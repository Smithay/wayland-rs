use std::sync::{Arc, Mutex};

use commons::{Implementation, Interface};
use protocol::wl_registry::{self, RequestsTrait};
use {NewProxy, Proxy};

struct Inner {
    list: Vec<(u32, String, u32)>,
    callback: Box<Implementation<Proxy<wl_registry::WlRegistry>, GlobalEvent> + Send>,
}

/// An utility to manage global objects
///
/// This utility provides an implemenation for the registry
/// that track the list of globals for you, as well as utilities
/// to bind them.
#[derive(Clone)]
pub struct GlobalManager {
    inner: Arc<Mutex<Inner>>,
    registry: Proxy<wl_registry::WlRegistry>,
}

/// An error that occured trying to bind a global
#[derive(Debug)]
pub enum GlobalError {
    /// The requested global was missing
    Missing,
    /// The global abvertized by the server has a lower version number
    /// than the one requested
    VersionTooLow(u32),
}

/// Event provided to the user callback of GlobalManager
pub enum GlobalEvent {
    /// A new global was created
    New {
        /// Id of the new global
        id: u32,
        /// Interface of the new global
        interface: String,
        /// Maximum supported version of the new global
        version: u32,
    },
    /// A global was removed
    Removed {
        /// Id of the removed global
        id: u32,
        /// Interface of the removed global
        interface: String,
    },
}

impl GlobalManager {
    /// Create a global manager handling a registry
    pub fn new(registry: NewProxy<wl_registry::WlRegistry>) -> GlobalManager {
        let inner = Arc::new(Mutex::new(Inner {
            list: Vec::new(),
            callback: Box::new(|_, _| {}),
        }));
        let inner_clone = inner.clone();

        let registry = registry.implement(move |msg, _proxy| {
            let mut inner = inner.lock().unwrap();
            match msg {
                wl_registry::Event::Global {
                    name,
                    interface,
                    version,
                } => {
                    inner.list.push((name, interface, version));
                }
                wl_registry::Event::GlobalRemove { name } => {
                    inner.list.retain(|&(n, _, _)| n != name);
                }
            }
        });

        GlobalManager {
            inner: inner_clone,
            registry: registry,
        }
    }

    /// Create a global manager handling a registry with a callback
    ///
    /// This global manager will track globals as a simple one, but will
    /// also forward the registry events to your callback.
    ///
    /// This can be used if you want to handle specially certain globals, but want
    /// to use the default mechanism for the rest.
    pub fn new_with_cb<Impl>(registry: NewProxy<wl_registry::WlRegistry>, callback: Impl) -> GlobalManager
    where
        Impl: Implementation<Proxy<wl_registry::WlRegistry>, GlobalEvent> + Send + 'static,
    {
        let inner = Arc::new(Mutex::new(Inner {
            list: Vec::new(),
            callback: Box::new(callback),
        }));
        let inner_clone = inner.clone();

        let registry = registry.implement(move |msg, proxy| {
            let mut inner = inner.lock().unwrap();
            match msg {
                wl_registry::Event::Global {
                    name,
                    interface,
                    version,
                } => {
                    inner.list.push((name, interface.clone(), version));
                    inner.callback.receive(
                        GlobalEvent::New {
                            id: name,
                            interface: interface,
                            version: version,
                        },
                        proxy,
                    );
                }
                wl_registry::Event::GlobalRemove { name } => {
                    if let Some((i, _)) = inner.list.iter().enumerate().find(|&(_, &(n, _, _))| n == name) {
                        let (id, interface, _) = inner.list.swap_remove(i);
                        inner.callback.receive(
                            GlobalEvent::Removed {
                                id: id,
                                interface: interface,
                            },
                            proxy,
                        );
                    } else {
                        panic!(
                            "Wayland protocol error: the server removed non-existing global \"{}\".",
                            name
                        );
                    }
                }
            }
        });

        GlobalManager {
            inner: inner_clone,
            registry: registry,
        }
    }

    /// Instanciate a global with highest available version
    ///
    /// This method is only appropriate for globals that are expected to
    /// not exist with multiplicity (sur as `wl_compositor` or `wl_shm`),
    /// as it will only bind a single one.
    pub fn instantiate_auto<I: Interface>(&self) -> Result<NewProxy<I>, GlobalError> {
        let inner = self.inner.lock().unwrap();
        for &(id, ref interface, version) in &inner.list {
            if interface == I::NAME {
                return Ok(self.registry.bind::<I>(version, id).unwrap());
            }
        }
        Err(GlobalError::Missing)
    }

    #[doc(hidden)]
    #[deprecated(since = "0.20.5", note = "Use the corrected `instantiate_auto` method instead.")]
    pub fn instanciate_auto<I: Interface>(&self) -> Result<NewProxy<I>, GlobalError> {
        self.instantiate_auto()
    }

    /// Instanciate a global with a specific version
    ///
    /// Like `instantiate_auto`, but will bind a specific version of
    /// this global an not the highest available.
    pub fn instantiate_exact<I: Interface>(&self, version: u32) -> Result<NewProxy<I>, GlobalError> {
        let inner = self.inner.lock().unwrap();
        for &(id, ref interface, server_version) in &inner.list {
            if interface == I::NAME {
                if version > server_version {
                    return Err(GlobalError::VersionTooLow(server_version));
                } else {
                    return Ok(self.registry.bind::<I>(version, id).unwrap());
                }
            }
        }
        Err(GlobalError::Missing)
    }

    #[doc(hidden)]
    #[deprecated(since = "0.20.5", note = "Use the corrected `instantiate_exact` method instead.")]
    pub fn instanciate_exact<I: Interface>(&self, version: u32) -> Result<NewProxy<I>, GlobalError> {
        self.instantiate_exact(version)
    }

    /// Retrieve the list of currently known globals
    pub fn list(&self) -> Vec<(u32, String, u32)> {
        self.inner.lock().unwrap().list.clone()
    }
}

/// Convenience macro to create a `GlobalManager` callback
///
/// This macro aims to simplify the specific but common case of
/// providing a callback to the `GlobalManager` that needs to
/// auto-bind all advertized instances of some specific globals
/// whenever they happen. Typically, your application will likely
/// want to keep track of all `wl_seat` and `wl_output` globals
/// to be able to correctly react to user input and their different
/// monitors.
///
/// The output of this macro is a closure, that can be given to
/// `GlobalManager::new_with_cb` as the callback argument.
///
/// Example use is typically:
///
/// ```no_run
/// # #[macro_use] extern crate wayland_client;
/// use wayland_client::GlobalManager;
/// # use wayland_client::{Display, NewProxy};
/// use wayland_client::protocol::wl_display::RequestsTrait;
/// use wayland_client::protocol::{wl_output, wl_seat};
///
/// # fn main() {
/// # let (display, mut event_queue) = Display::connect_to_env().unwrap();
/// # let seat_callback: fn(Result<NewProxy<_>,_>, ()) = unimplemented!();
/// # let output_callback: fn(Result<NewProxy<_>,_>, ()) = unimplemented!();
/// let globals = GlobalManager::new_with_cb(
///     display.get_registry().unwrap(),
///     global_filter!(
///         // Bind all wl_seat with version 4
///         [wl_seat::WlSeat, 4, seat_callback],
///         // Bind all wl_output with version 1
///         [wl_output::WlOutput, 1, output_callback]
///     )
/// );
/// # }
/// ```
///
/// The supplied callbacks for each global kind must be an instance of a type
/// implementing the `Interface<Result<NewProxy<I>, u32>, ()>` trait. The
/// argument provided to your callback is a result containing the `NewProxy`
/// of the newly instantiated global on success. The error case happens if the
/// server advertized a lower version of the global than the one you requested,
/// in which case you are given the version it advertized.
///
/// As with all implementations, you can also provide closures for the various
/// callbacks. However, due to a lack of capability of rustc's inference, you'll
/// likely need to add some type annotation to your closure, typically something
/// like
///
/// ```ignore
/// global_filer!(
///     [Interface, version, |new_proxy: Result<NewProxy<_>, _>, ()| {
///         ...
///     }]
/// );
/// ```
#[macro_export]
macro_rules! global_filter {
    ($([$interface:ty, $version:expr, $callback:expr]),*) => {
        {
            use $crate::commons::{Implementation, Interface};
            use $crate::protocol::wl_registry::{self, RequestsTrait};
            use $crate::{Proxy, GlobalEvent, NewProxy};
            type Callback = Box<Implementation<Proxy<wl_registry::WlRegistry>, (u32, u32)> + Send>;
            let mut callbacks: Vec<(&'static str, Callback)> = Vec::new();
            // Create the callback list
            $({
                let mut cb = { $callback };
                fn typecheck<Impl: Implementation<(), Result<NewProxy<$interface>, u32>>>(_: &Impl) {}
                typecheck(&cb);
                callbacks.push((
                    <$interface as Interface>::NAME,
                    Box::new(move |(id, version), registry: Proxy<wl_registry::WlRegistry>| {
                        if version < $version {
                            cb.receive(Err(version), ())
                        } else {
                            cb.receive(Ok(registry.bind::<$interface>(version, id).unwrap()), ())
                        }
                    }) as Box<_>
                ));
            })*

            // return the global closure
            move |event: GlobalEvent, registry: Proxy<wl_registry::WlRegistry>| {
                if let GlobalEvent::New { id, interface, version } = event {
                    for &mut (iface, ref mut cb) in &mut callbacks {
                        if iface == interface {
                            cb.receive((id, version), registry);
                            break;
                        }
                    }
                }
            }
        }
    }
}
