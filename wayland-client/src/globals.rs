use std::sync::{Arc, Mutex};

use protocol::wl_display::{self, RequestsTrait as DisplayRequests};
use protocol::wl_registry::{self, RequestsTrait as RegistryRequests};
use {Interface, NewProxy, Proxy};

struct Inner {
    list: Vec<(u32, String, u32)>,
    callback: Box<FnMut(GlobalEvent, Proxy<wl_registry::WlRegistry>) + Send>,
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

/// An error that occurred trying to bind a global
#[derive(Debug, PartialEq)]
pub enum GlobalError {
    /// The requested global was missing
    Missing,
    /// The global advertised by the server has a lower version number
    /// than the one requested
    VersionTooLow(u32),
}

impl ::std::error::Error for GlobalError {
    fn description(&self) -> &str {
        match *self {
            GlobalError::Missing => "The requested global was missing.",
            GlobalError::VersionTooLow(_) => "The requested global's version is too low.",
        }
    }
}

impl ::std::fmt::Display for GlobalError {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> Result<(), ::std::fmt::Error> {
        f.write_str(::std::error::Error::description(self))
    }
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
    pub fn new(display: &Proxy<wl_display::WlDisplay>) -> GlobalManager {
        let inner = Arc::new(Mutex::new(Inner {
            list: Vec::new(),
            callback: Box::new(|_, _| {}),
        }));
        let inner_clone = inner.clone();

        let registry = display
            .get_registry(|registry| {
                registry.implement(
                    move |msg, _proxy| {
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
                    },
                    (),
                )
            })
            .expect("Attempted to create a GlobalManager from a dead display.");

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
    pub fn new_with_cb<F>(display: &Proxy<wl_display::WlDisplay>, callback: F) -> GlobalManager
    where
        F: FnMut(GlobalEvent, Proxy<wl_registry::WlRegistry>) + Send + 'static,
    {
        let inner = Arc::new(Mutex::new(Inner {
            list: Vec::new(),
            callback: Box::new(callback),
        }));
        let inner_clone = inner.clone();

        let registry = display
            .get_registry(|registry| {
                registry.implement(
                    move |msg, proxy| {
                        let mut inner = inner.lock().unwrap();
                        let inner = &mut *inner;
                        match msg {
                            wl_registry::Event::Global {
                                name,
                                interface,
                                version,
                            } => {
                                inner.list.push((name, interface.clone(), version));
                                (inner.callback)(
                                    GlobalEvent::New {
                                        id: name,
                                        interface: interface,
                                        version: version,
                                    },
                                    proxy,
                                );
                            }
                            wl_registry::Event::GlobalRemove { name } => {
                                if let Some((i, _)) =
                                    inner.list.iter().enumerate().find(|&(_, &(n, _, _))| n == name)
                                {
                                    let (id, interface, _) = inner.list.swap_remove(i);
                                    (inner.callback)(
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
                    },
                    (),
                )
            })
            .expect("Attempted to create a GlobalManager from a dead display.");

        GlobalManager {
            inner: inner_clone,
            registry: registry,
        }
    }

    /// Instantiate a global with highest available version
    ///
    /// This method is only appropriate for globals that are expected to
    /// not exist with multiplicity (such as `wl_compositor` or `wl_shm`),
    /// as it will only bind a single one.
    pub fn instantiate_auto<I: Interface, F>(&self, implementor: F) -> Result<Proxy<I>, GlobalError>
    where
        F: FnOnce(NewProxy<I>) -> Proxy<I>,
    {
        let inner = self.inner.lock().unwrap();
        for &(id, ref interface, version) in &inner.list {
            if interface == I::NAME {
                return Ok(self.registry.bind(version, id, implementor).unwrap());
            }
        }
        Err(GlobalError::Missing)
    }

    /// Instantiate a global with a specific version
    ///
    /// Like `instantiate_auto`, but will bind a specific version of
    /// this global an not the highest available.
    pub fn instantiate_exact<I: Interface, F>(
        &self,
        version: u32,
        implementor: F,
    ) -> Result<Proxy<I>, GlobalError>
    where
        F: FnOnce(NewProxy<I>) -> Proxy<I>,
    {
        let inner = self.inner.lock().unwrap();
        for &(id, ref interface, server_version) in &inner.list {
            if interface == I::NAME {
                if version > server_version {
                    return Err(GlobalError::VersionTooLow(server_version));
                } else {
                    return Ok(self.registry.bind(version, id, implementor).unwrap());
                }
            }
        }
        Err(GlobalError::Missing)
    }

    /// Retrieve the list of currently known globals
    pub fn list(&self) -> Vec<(u32, String, u32)> {
        self.inner.lock().unwrap().list.clone()
    }
}

/// A trait for implementation of the global advertisement
///
/// It is automatically implemented for `FnMut(NewProxy<I>) -> Proxy<I>`
/// closures, in which case the `error` messages are ignored.
pub trait GlobalImplementor<I: Interface> {
    /// A new global of given interface has been instantiated and you are
    /// supposed to provide an implementation for it.
    fn new_global(&mut self, global: NewProxy<I>) -> Proxy<I>;
    /// A global was advertised but its version was lower than the minimal version
    /// you requested.
    ///
    /// The advertised version is provided as argument.
    fn error(&mut self, _version: u32) {}
}

impl<F, I: Interface> GlobalImplementor<I> for F
where
    F: FnMut(NewProxy<I>) -> Proxy<I>,
{
    fn new_global(&mut self, global: NewProxy<I>) -> Proxy<I> {
        (*self)(global)
    }
}

/// Convenience macro to create a `GlobalManager` callback
///
/// This macro aims to simplify the specific but common case of
/// providing a callback to the `GlobalManager` that needs to
/// auto-bind all advertised instances of some specific globals
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
/// # use wayland_client::{Display, NewProxy, Proxy};
/// use wayland_client::protocol::{wl_output, wl_seat};
///
/// # fn main() {
/// # let (display, mut event_queue) = Display::connect_to_env().unwrap();
/// # let seat_implementor: fn(NewProxy<_>) -> Proxy<_> = unimplemented!();
/// # let output_implementor: fn(NewProxy<_>) -> Proxy<_> = unimplemented!();
/// let globals = GlobalManager::new_with_cb(
///     &display,
///     global_filter!(
///         // Bind all wl_seat with version 4
///         [wl_seat::WlSeat, 4, seat_implementor],
///         // Bind all wl_output with version 1
///         [wl_output::WlOutput, 1, output_implementor]
///     )
/// );
/// # }
/// ```
///
/// The supplied callbacks for each global kind must be an instance of a type
/// implementing the `GlobalImplementor<I>` trait. The argument provided to your
/// callback is a `NewProxy`  of the newly instantiated global, and you should implement
/// it and return the implemented proxy. The error case happens if the server advertised
/// a lower version of the global than the one you requested, in which case you are given
/// the version it advertised in the error method, if you want to handle it graciously.
///
/// As with all implementations, you can also provide closures for the various
/// callbacks, in this case the errors will be ignored. However, due to a lack of
/// capability of rustc's inference, you'll likely need to add some type annotation
/// to your closure, typically something like
///
/// ```ignore
/// global_filter!(
///     [Interface, version, |new_proxy: NewProxy<_>| {
///         ...
///     }]
/// );
/// ```
#[macro_export]
macro_rules! global_filter {
    ($([$interface:ty, $version:expr, $callback:expr]),*) => {
        {
            use $crate::protocol::wl_registry::{self, RequestsTrait};
            use $crate::{Proxy, GlobalEvent, NewProxy, Interface, GlobalImplementor};
            type Callback = Box<FnMut(u32, u32, Proxy<wl_registry::WlRegistry>) + Send>;
            let mut callbacks: Vec<(&'static str, Callback)> = Vec::new();
            // Create the callback list
            $({
                let mut cb = { $callback };
                callbacks.push((
                    <$interface as Interface>::NAME,
                    Box::new(move |id, version, registry: Proxy<wl_registry::WlRegistry>| {
                        if version < $version {
                            GlobalImplementor::<$interface>::error(&mut cb, version);
                        } else {
                            registry.bind::<$interface, _>(
                                version,
                                id,
                                |newp| GlobalImplementor::<$interface>::new_global(&mut cb, newp)
                            );
                        }
                    }) as Box<_>
                ));
            })*

            // return the global closure
            move |event: GlobalEvent, registry: Proxy<wl_registry::WlRegistry>| {
                if let GlobalEvent::New { id, interface, version } = event {
                    for &mut (iface, ref mut cb) in &mut callbacks {
                        if iface == interface {
                            cb(id, version, registry);
                            break;
                        }
                    }
                }
            }
        }
    }
}
