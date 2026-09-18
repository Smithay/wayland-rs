//! Helpers for handling the initialization of an app
//!
//! At the startup of your Wayland app, the initial step is generally to retrieve the list of globals
//! advertized by the compositor from the registry. Using the [`Dispatch`] mechanism for this task can be
//! very unpractical, this is why this module provides a special helper for handling the registry.
//!
//! The entry point of this helper is the [`GlobalList::init`] function. Given a reference to your
//! [`Connection`] and a [`QueueHandle`], retrieve the initial list of globals, and register a
//! handler using your provided `Dispatch<WlRegistry,_>` implementation for handling dynamic registry events.
//!
//! ## Example
//!
//! ```no_run
//! use wayland_client::{
//!     Connection, Dispatch, QueueHandle,
//!     globals::{Global, GlobalList, GlobalListHandler},
//!     protocol::{wl_registry, wl_compositor},
//! };
//! # use std::sync::Mutex;
//! # struct State;
//!
//! // You need to provide a GlobalListHandler impl for your app
//! impl GlobalListHandler for State {
//!     /* react to dynamic global events here */
//! }
//!
//! let conn = unsafe { Connection::connect_to_env() }.unwrap();
//! let mut queue = conn.new_event_queue();
//! let globals = GlobalList::init(&conn, &queue.handle()).unwrap();
//!
//! # impl wayland_client::Dispatch<wl_compositor::WlCompositor, State> for () {
//! #     fn event(
//! #         &self,
//! #         state: &mut State,
//! #         proxy: &wl_compositor::WlCompositor,
//! #         event: wl_compositor::Event,
//! #         conn: &Connection,
//! #         qh: &QueueHandle<State>,
//! #     ) {}
//! # }
//! // now you can bind the globals you need for your app
//! let compositor: wl_compositor::WlCompositor = globals.bind_singleton(4..=5, &queue.handle(), ()).unwrap();
//! ```

use std::{
    fmt,
    ops::RangeInclusive,
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicBool, Ordering},
    },
};

use wayland_backend::{
    client::{Backend, InvalidId, ObjectData, ObjectId, WaylandError},
    protocol::{Interface, OwnedMessage},
};

use crate::{
    Connection, Dispatch, Proxy, QueueHandle,
    protocol::{wl_display, wl_fixes, wl_registry},
};

/// Handler for runtime global addition/removal in [`GlobalList`] created with
/// [`GlobalList::init`]
pub trait GlobalListHandler: Sized {
    /// A global has been added dynamically after creation of the [`GlobalList`]
    ///
    /// By default, does nothing.
    fn runtime_add_global(
        &mut self,
        _globals: &GlobalList,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _global: &Global,
    ) {
    }

    /// A global has been removed
    ///
    /// By default, does nothing.
    fn runtime_remove_global(
        &mut self,
        _globals: &GlobalList,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _global: &Global,
    ) {
    }
}

/// A helper for global initialization.
///
/// See [the module level documentation][self] for more.
#[derive(Clone, Debug)]
pub struct GlobalList {
    registry: wl_registry::WlRegistry,
}

impl GlobalList {
    /// Initialize registry and retrieve the initial list of globals
    ///
    /// See [the module level documentation][self] for more.
    pub fn init<State>(
        conn: &Connection,
        qh: &QueueHandle<State>,
    ) -> Result<GlobalList, GlobalError>
    where
        State: GlobalListHandler + 'static,
    {
        let display = conn.display();
        let fixes = OnceLock::<wl_fixes::WlFixes>::new();

        let data = Arc::new(RegistryState {
            data: GlobalListData { contents: Default::default(), fixes },
            handle: qh.clone(),
            initial_roundtrip_done: AtomicBool::new(false),
        });
        let registry =
            display.send_constructor(wl_display::Request::GetRegistry {}, data.clone())?;
        // We don't need to dispatch the event queue as for now nothing will be sent to it
        conn.roundtrip()?;
        data.initial_roundtrip_done.store(true, Ordering::Relaxed);
        Ok(GlobalList { registry })
    }

    fn data(&self) -> &GlobalListData {
        self.registry.data::<GlobalListData>().unwrap()
    }

    /// Get a copy of the contents of the list of globals.
    pub fn clone_list(&self) -> Vec<Global> {
        self.data().contents.lock().unwrap().clone()
    }

    /// Binds a global, returning a new protocol object associated with the global.
    ///
    /// The `version` specifies the range of versions that should be bound. This function will guarantee the
    /// version of the returned protocol object is the lower of the maximum requested version and the advertised
    /// version.
    ///
    /// If the lower bound of the `version` is greater than the version advertised by the server, then
    /// [`BindError::UnsupportedVersion`] is returned.
    ///
    /// ## Multi-instance/Device globals.
    ///
    /// This function is not intended to be used with globals that have multiple instances such as `wl_output`
    /// and `wl_seat`. These types of globals need their own initialization mechanism because these
    /// multi-instance globals may be removed at runtime. To handle then, you should instead call
    /// [`Self::bind_specific`] in the [`GlobalListHandler`] of your `State`.
    ///
    /// # Panics
    ///
    /// This function will panic if the maximum requested version is greater than the known maximum version of
    /// the interface. The known maximum version is determined by the code generated using wayland-scanner.
    pub fn bind_singleton<I, State, U>(
        &self,
        version: RangeInclusive<u32>,
        qh: &QueueHandle<State>,
        udata: U,
    ) -> Result<I, BindError>
    where
        I: Proxy + 'static,
        State: 'static,
        U: Dispatch<I, State> + Send + Sync + 'static,
    {
        let interface = I::interface();
        assert_valid_interface_version(&version, interface);

        let guard = self.data().contents.lock().unwrap();
        let global = guard
            .iter()
            // Find the global with the correct interface
            .find(|Global { interface: interface_name, .. }| interface.name == interface_name)
            .ok_or(BindError::NotPresent(interface.name))?;

        self.bind_inner(global, version, qh, udata)
    }

    /// Binds all globals with a given interface.
    ///
    /// Typically for globals with multiple instances, this should be called at start,
    /// globals added later should be handled in [`GlobalListHandler::runtime_add_global`]
    /// using `[Self::bind_specific]`.
    pub fn bind_all<I, State, U>(
        &self,
        version: std::ops::RangeInclusive<u32>,
        qh: &QueueHandle<State>,
        mut make_udata: impl FnMut(&Global) -> U,
    ) -> Result<Vec<I>, BindError>
    where
        I: Proxy + 'static,
        State: 'static,
        U: Dispatch<I, State> + Send + Sync + 'static,
    {
        let interface = I::interface();
        assert_valid_interface_version(&version, interface);

        let guard = self.data().contents.lock().unwrap();
        guard
            .iter()
            .filter(|global| global.interface == interface.name)
            .map(|global| self.bind_inner(global, version.clone(), qh, make_udata(global)))
            .collect()
    }

    /// Binds a global, returning a new object associated with the global.
    ///
    /// This binds a specific object by its name.
    ///
    /// Typically, this should be called in [`GlobalListHandler::runtime_add_global`] for dynamically
    /// added globals.
    pub fn bind_specific<I, State, U>(
        &self,
        name: u32,
        version: std::ops::RangeInclusive<u32>,
        qh: &QueueHandle<State>,
        udata: U,
    ) -> Result<I, BindError>
    where
        I: Proxy + 'static,
        State: 'static,
        U: Dispatch<I, State> + Send + Sync + 'static,
    {
        let interface = I::interface();
        assert_valid_interface_version(&version, interface);

        let guard = self.data().contents.lock().unwrap();
        let global = guard
            .iter()
            // Optimize for `runtime_add_global` which will use the last entry
            .rev()
            // Find the global with correct name and interface
            .find(|global| global.name == name && global.interface == interface.name)
            // TODO Error for not finding name, rather than interface?
            .ok_or(BindError::NotPresent(interface.name))?;

        self.bind_inner(global, version, qh, udata)
    }

    fn bind_inner<I, State, U>(
        &self,
        global: &Global,
        version: RangeInclusive<u32>,
        qh: &QueueHandle<State>,
        udata: U,
    ) -> Result<I, BindError>
    where
        I: Proxy + 'static,
        State: 'static,
        U: Dispatch<I, State> + Send + Sync + 'static,
    {
        // Test version requirements
        if *version.start() > global.version {
            return Err(BindError::UnsupportedVersion {
                interface: I::interface().name,
                requested: *version.start(),
                available: global.version,
            });
        }

        // To get the version to bind, take the lower of the version advertised by the server and the maximum
        // requested version.
        let negotiated_version = global.version.min(*version.end());

        Ok(self.registry.bind(global.name, negotiated_version, qh, udata))
    }

    /// Returns the [`WlRegistry`][wl_registry] protocol object.
    ///
    /// This may be used if more direct control when creating globals is needed.
    pub fn registry(&self) -> &wl_registry::WlRegistry {
        &self.registry
    }

    /// Tries to destroy the [`WlRegistry`][wl_registry] protocol object.
    ///
    /// If successful no new events will be emitted and the `GlobalListContent`
    /// will not be updated anymore. Other proocol objects are not affected.
    ///
    /// This might end up doing nothing if the compositor doesn't support `wl_fixes`
    /// in which case the registry cannot be destroyed without closing the connection.
    pub fn destroy(self) {
        if let Some(fixes) = self.data().fixes.get() {
            let id = self.registry.id();
            fixes.destroy_registry(&self.registry);
            if let Some(backend) = fixes.backend().upgrade() {
                backend.destroy_object(id).unwrap();
            }
            fixes.destroy();
        }
    }
}

/// An error that may occur when initializing the global list.
#[derive(Debug)]
pub enum GlobalError {
    /// The backend generated an error
    Backend(WaylandError),

    /// An invalid object id was acted upon.
    InvalidId(InvalidId),
}

impl std::error::Error for GlobalError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            GlobalError::Backend(source) => Some(source),
            GlobalError::InvalidId(source) => std::error::Error::source(source),
        }
    }
}

impl std::fmt::Display for GlobalError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            GlobalError::Backend(source) => {
                write!(f, "Backend error: {source}")
            }
            GlobalError::InvalidId(source) => write!(f, "{source}"),
        }
    }
}

impl From<WaylandError> for GlobalError {
    fn from(source: WaylandError) -> Self {
        GlobalError::Backend(source)
    }
}

impl From<InvalidId> for GlobalError {
    fn from(source: InvalidId) -> Self {
        GlobalError::InvalidId(source)
    }
}

/// An error that occurs when a binding a global fails.
#[derive(Debug)]
pub enum BindError {
    /// The requested version of the global is not supported.
    UnsupportedVersion {
        /// The name of the global for which the server provides a too low value.
        interface: &'static str,
        /// The lowest version that was requested by the caller, must be greater than [`Self::UnsupportedVersion::requested`].
        requested: u32,
        /// The actual verison that was available on the server, must be less than [`Self::UnsupportedVersion::requested`].
        available: u32,
    },

    /// The requested global was not found in the registry.
    NotPresent(&'static str),
}

impl std::error::Error for BindError {}

impl fmt::Display for BindError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            BindError::UnsupportedVersion { interface, requested, available } => {
                write!(
                    f,
                    "the requested version `{requested}` of the global `{interface}` is not supported, only `{available}` is available"
                )
            }
            BindError::NotPresent(name) => {
                write!(f, "the requested global `{name}` was not found in the registry")
            }
        }
    }
}

/// Description of a global.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Global {
    /// The name of the global.
    ///
    /// This is an identifier used by the server to reference some specific global.
    pub name: u32,
    /// The interface of the global.
    ///
    /// This describes what type of protocol object the global is.
    pub interface: String,
    /// The advertised version of the global.
    ///
    /// This specifies the maximum version of the global that may be bound. This means any lower version of
    /// the global may be bound.
    pub version: u32,
}

#[derive(Debug)]
struct GlobalListData {
    contents: Mutex<Vec<Global>>,
    fixes: OnceLock<wl_fixes::WlFixes>,
}

impl GlobalListData {
    fn add(&self, global: Global) {
        self.contents.lock().unwrap().push(global);
    }

    fn remove(&self, name: u32) -> Option<Global> {
        let mut guard = self.contents.lock().unwrap();
        let idx = guard.iter().position(|i| i.name == name)?;
        Some(guard.remove(idx))
    }
}

impl<D> Dispatch<wl_registry::WlRegistry, D> for GlobalListData
where
    D: GlobalListHandler,
{
    fn event(
        &self,
        state: &mut D,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        let globals = GlobalList { registry: registry.clone() };
        match event {
            wl_registry::Event::Global { name, interface, version } => {
                let global = Global { name, interface, version };
                self.add(global.clone());
                state.runtime_add_global(&globals, conn, qh, &global);
            }
            wl_registry::Event::GlobalRemove { name } => {
                if let Some(global) = self.remove(name) {
                    state.runtime_remove_global(&globals, conn, qh, &global);
                }
            }
        }
    }
}

struct RegistryState<State> {
    data: GlobalListData,
    handle: QueueHandle<State>,
    initial_roundtrip_done: AtomicBool,
}

impl<State> ObjectData for RegistryState<State>
where
    State: GlobalListHandler + 'static,
{
    fn event(
        self: Arc<Self>,
        backend: &Backend,
        msg: OwnedMessage<ObjectId>,
    ) -> Option<Arc<dyn ObjectData>> {
        // For initial roundtrip, update immediately without waiting for dispatch.
        // So globals are available after `GlobalList::init` returns.
        // later, handle in `Dispatch` implementation.
        if !self.initial_roundtrip_done.load(Ordering::Relaxed) {
            let conn = Connection::from_backend(backend.clone());
            // Can't do much if the server sends a malformed message
            if let Ok((registry, event)) = wl_registry::WlRegistry::parse_event(&conn, msg) {
                match event {
                    wl_registry::Event::Global { name, interface, version } => {
                        let wl_fixes_ver = 1u32..=1;
                        if interface == "wl_fixes" && version >= *wl_fixes_ver.start() {
                            let _ = self.data.fixes.set(registry.bind(
                                name,
                                version.min(*wl_fixes_ver.end()),
                                &self.handle,
                                crate::Noop,
                            ));
                        }

                        self.data.add(Global { name, interface, version });
                    }

                    wl_registry::Event::GlobalRemove { name: remove } => {
                        self.data.remove(remove);
                    }
                }
            };
        } else {
            // forward the message to the event queue as normal
            self.handle
                .inner
                .lock()
                .unwrap()
                .enqueue_event::<wl_registry::WlRegistry, GlobalListData>(msg, self.clone())
        }

        // We do not create any objects in this event handler.
        None
    }

    fn destroyed(&self, _id: &ObjectId) {}

    fn data_as_any(&self) -> &dyn std::any::Any {
        &self.data
    }
}

fn assert_valid_interface_version(version: &RangeInclusive<u32>, interface: &'static Interface) {
    if *version.end() > interface.version {
        // This is a panic because it's a compile-time programmer error, not a runtime error.
        panic!(
            "Maximum version ({}) of {} was higher than the proxy's maximum version ({}); outdated wayland XML files?",
            version.end(),
            interface.name,
            interface.version
        );
    }
}
