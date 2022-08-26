//! Helpers for initial global initialization.
//!
//! wayland-client has an API designed around the [`Dispatch`] trait. Typically an implementation of the
//! [`Dispatch`] trait for [`WlRegistry`](wl_registry::WlRegistry) would be used in order to determine what
//! globals are available.
//!
//! However when using a delegate type is used, the delegate type may need to know what globals are available.
//! This causes a chicken and egg problem since initializing the global list requires dispatching the state,
//! but the delegate types that are part of the state require the global list.
//!
//! This can be worked around with an internal [`Option`] that indicates whether the type's internal global
//! has been initialized but that then requires handling many runtime checks for globals an application requires
//! to start.
//!
//! [`GlobalList`] provides a way to get an initial list of globals during application initialization without
//! needing to create a state.
//!
//! Getting a global list only requires a [`Connection`]
//!
//! ```no_run
//! use wayland_client::Connection;
//! use wayland_client::global::GlobalList;
//!
//! let conn = Connection::connect_to_env().unwrap();
//! let globals = GlobalList::new(&conn).unwrap();
//!
//! // Initialize the state, creating any required globals.
//! ```

use std::{
    ops::RangeInclusive,
    sync::{Arc, Mutex},
};

use wayland_backend::{
    client::{Backend, InvalidId, ObjectData, ObjectId, WaylandError},
    protocol::Message,
};

use crate::{
    protocol::{wl_display, wl_registry},
    Connection, Dispatch, Proxy, QueueHandle,
};

/// A helper for global initialization.
///
/// See [the module level documentation](self) for more.
#[derive(Debug)]
pub struct GlobalList {
    registry: wl_registry::WlRegistry,
}

impl GlobalList {
    /// Gets the list of globals available to the connection.
    pub fn new(conn: &Connection) -> Result<Self, GlobalError> {
        let display = conn.display();
        let data = RegistryState { globals: Mutex::new(Vec::new()) };

        // Initialize the registry using the lower level wayland-backend API.
        let registry =
            conn.send_request(&display, wl_display::Request::GetRegistry {}, Some(Arc::new(data)))?;
        let registry = wl_registry::WlRegistry::from_id(conn, registry)?;

        // Roundtrip to wait until the server has send all globals over.
        conn.roundtrip()?;

        Ok(Self { registry })
    }

    /// Returns the list of advertised globals.
    ///
    /// This list is only valid until the next time the events are processed from the connection.
    pub fn globals(&self) -> Vec<Global> {
        let state = self.registry.data::<RegistryState>().unwrap();
        let guard = state.globals.lock().unwrap();
        guard.clone()
    }

    /// Binds a global, returning a new protocol object associated with the global.
    ///
    /// The `version` specifies the range of versions that should be bound. This function will guarantee the
    /// version of the returned protocol object is the lower of the maximum requested version and the advertised
    /// version.
    ///
    /// If the lower bound of the `version` is less than the version advertised by the server, then
    /// [`BindError::UnsupportedVersion`] is returned.
    ///
    /// ## Multi-instance/Device globals.
    ///
    /// This function is not intended to be used with globals that have multiple instances such as `wl_output`
    /// and `wl_seat`. These types of globals need their own initialization mechanism because these
    /// multi-instance globals may be removed at runtime.
    ///
    /// # Panics
    ///
    /// This function will panic if the maximum requested version is greater than the known maximum version of
    /// the interface. The known maximum version is determined by the code generated using wayland-scanner.
    pub fn bind_one<I, State, U>(
        &self,
        qh: &QueueHandle<State>,
        version: RangeInclusive<u32>,
        udata: U,
    ) -> Result<I, BindError>
    where
        I: Proxy + 'static,
        State: Dispatch<I, U> + 'static,
        U: Send + Sync + 'static,
    {
        let version_start = *version.start();
        let version_end = *version.end();
        let interface = I::interface();

        if *version.end() > interface.version {
            // This is a panic because it's a compile-time programmer error, not a runtime error.
            panic!("Maximum version ({}) of {} was higher than the proxy's maximum version ({}); outdated wayland XML files?",
                version.end(), interface.name, interface.version);
        }

        let state = self.registry.data::<RegistryState>().unwrap();
        let guard = state.globals.lock().unwrap();
        let (name, version) = guard
            .iter()
            // Find the with the correct interface
            .filter_map(|Global { name, interface: interface_name, version }| {
                // TODO: then_some
                if interface.name == &interface_name[..] {
                    Some((*name, *version))
                } else {
                    None
                }
            })
            .next()
            .ok_or(BindError::NotPresent)?;

        // Test version requirements
        if version < version_start {
            return Err(BindError::UnsupportedVersion);
        }

        // To get the version to bind, take the lower of the version advertised by the server and the maximum
        // requested version.
        let version = version.min(version_end);

        Ok(self.registry.bind(name, version, qh, udata))
    }

    /// Returns the [`WlRegistry`](wl_registry) protocol object.
    ///
    /// This may be used if more direct control when creating globals is needed.
    pub fn registry(&self) -> &wl_registry::WlRegistry {
        &self.registry
    }
}

/// An error that may occur when initializing the global list.
#[derive(Debug, thiserror::Error)]
pub enum GlobalError {
    /// The backend generated an error
    #[error("Backend error: {0}")]
    Backend(#[from] WaylandError),

    /// An invalid object id was acted upon.
    #[error(transparent)]
    InvalidId(#[from] InvalidId),
}

/// An error that occurs when a binding a global fails.
#[derive(Debug, thiserror::Error)]
pub enum BindError {
    /// The requested version of the global is not supported.
    #[error("the requested version of the global is not supported")]
    UnsupportedVersion,

    /// The requested global was not found in the registry.
    #[error("the requested global was not found in the registry")]
    NotPresent,
}

/// Description of a global.
#[derive(Debug, Clone)]
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

struct RegistryState {
    globals: Mutex<Vec<Global>>,
}

impl ObjectData for RegistryState {
    fn event(
        self: Arc<Self>,
        backend: &Backend,
        msg: Message<ObjectId>,
    ) -> Option<Arc<dyn ObjectData>> {
        let conn = Connection::from_backend(backend.clone());

        // Can't do much if the server sends a malformed message
        if let Ok((_, event)) = wl_registry::WlRegistry::parse_event(&conn, msg) {
            match event {
                wl_registry::Event::Global { name, interface, version } => {
                    let mut guard = self.globals.lock().unwrap();
                    guard.push(Global { name, interface, version });
                }

                wl_registry::Event::GlobalRemove { name: remove } => {
                    let mut guard = self.globals.lock().unwrap();
                    guard.retain(|Global { name, .. }| name != &remove);
                }
            }
        };

        // We do not create any objects in this event handler.
        None
    }

    fn destroyed(&self, _id: ObjectId) {
        // A registry cannot be destroyed unless disconnected.
    }
}
