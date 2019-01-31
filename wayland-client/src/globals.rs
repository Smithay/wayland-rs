use std::sync::{Arc, Mutex};

use protocol::wl_display;
use protocol::wl_registry;
use {Interface, NewProxy, Proxy};

struct Inner {
    list: Vec<(u32, String, u32)>,
}

/// An utility to manage global objects
///
/// This utility provides an implemenation for the registry
/// that track the list of globals for you, as well as utilities
/// to bind them.
#[derive(Clone)]
pub struct GlobalManager {
    inner: Arc<Mutex<Inner>>,
    registry: wl_registry::WlRegistry,
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
    ///
    /// In order to use `GlobalManager` from a different thread than the one `display` was created
    /// on, wrap the `display` to an `EventQueue` on the appropriate thread using
    /// `Proxy::make_wrapper()` before calling this function.
    pub fn new(display: &wl_display::WlDisplay) -> GlobalManager {
        let inner = Arc::new(Mutex::new(Inner { list: Vec::new() }));
        let inner_clone = inner.clone();

        let registry = display
            .get_registry(|registry| {
                registry.implement_closure(
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
                            _ => {}
                        }
                    },
                    (),
                )
            })
            .expect("Attempted to create a GlobalManager from a dead display.");

        GlobalManager {
            inner: inner_clone,
            registry,
        }
    }

    /// Create a global manager handling a registry with a callback
    ///
    /// This global manager will track globals as a simple one, but will
    /// also forward the registry events to your callback.
    ///
    /// This can be used if you want to handle specially certain globals, but want
    /// to use the default mechanism for the rest.
    ///
    /// In order to use `GlobalManager` from a different thread than the one `display` was created
    /// on, wrap the `display` to an `EventQueue` on the appropriate thread using
    /// `Proxy::make_wrapper()` before calling this function.
    pub fn new_with_cb<F>(display: &wl_display::WlDisplay, mut callback: F) -> GlobalManager
    where
        F: FnMut(GlobalEvent, wl_registry::WlRegistry) + 'static,
    {
        let inner = Arc::new(Mutex::new(Inner { list: Vec::new() }));
        let inner_clone = inner.clone();

        let registry = display
            .get_registry(|registry| {
                registry.implement_closure(
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
                                callback(
                                    GlobalEvent::New {
                                        id: name,
                                        interface,
                                        version,
                                    },
                                    proxy,
                                    );
                            }
                            wl_registry::Event::GlobalRemove { name } => {
                                if let Some((i, _)) =
                                    inner.list.iter().enumerate().find(|&(_, &(n, _, _))| n == name)
                                    {
                                        let (id, interface, _) = inner.list.swap_remove(i);
                                        callback(
                                            GlobalEvent::Removed {
                                                id,
                                                interface,
                                            },
                                            proxy,
                                            );
                                    } else {
                                        panic!(
                                            "Wayland protocol error: the server removed non-existing global \"{}\".",
                                            name
                                        );
                                    }
                            },
                            _ => {}
                        }
                    },
                    (),
                )
            })
            .expect("Attempted to create a GlobalManager from a dead display.");

        GlobalManager {
            inner: inner_clone,
            registry,
        }
    }

    /// Instantiate a global with a specific version
    ///
    /// Meaning of requests and events can change depending on the object version you use,
    /// as such unless you specifically want to support several versions of a protocol, it is
    /// recommended to use this method with an hardcoded value for the version (the one you'll
    /// use a as reference for your implementation). Notably you should *not* use `I::VERSION`
    /// as a version, as this value can change when the protocol files are updated.
    ///
    /// This method is only appropriate for globals that are expected to
    /// not exist with multiplicity (such as `wl_compositor` or `wl_shm`),
    /// as it will always bind the first one that was advertized.
    pub fn instantiate_exact<I, F>(&self, version: u32, implementor: F) -> Result<I, GlobalError>
    where
        I: Interface + From<Proxy<I>>,
        F: FnOnce(NewProxy<I>) -> I,
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

    /// Instantiate a global from a version range
    ///
    /// If you want to support several versions of a particular global, this method allows you to
    /// specify a range of versions that you accept. It'll bind the highest possible version that
    /// is between `min_version` and `max_version` inclusive, and return an error if the highest
    /// version supported by the compositor is lower than `min_version`. As for
    /// `instantiate_exact`, you should not use `I::VERSION` here: the versions your code support
    /// do not change when the protocol files are updated.
    ///
    /// When trying to support several versions of a protocol, you can check which version has
    /// actually been used on any object using the `Proxy::version()` method.
    ///
    /// As `instantiate_exact`, it should only be used for singleton globals, for the same reasons.
    pub fn instantiate_range<I, F>(
        &self,
        min_version: u32,
        max_version: u32,
        implementor: F,
    ) -> Result<I, GlobalError>
    where
        I: Interface + From<Proxy<I>>,
        F: FnOnce(NewProxy<I>) -> I,
    {
        let inner = self.inner.lock().unwrap();
        for &(id, ref interface, version) in &inner.list {
            if interface == I::NAME {
                if version >= min_version {
                    return Ok(self
                        .registry
                        .bind(::std::cmp::min(version, max_version), id, implementor)
                        .unwrap());
                } else {
                    return Err(GlobalError::VersionTooLow(version));
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
    fn new_global(&mut self, global: NewProxy<I>) -> I;
    /// A global was advertised but its version was lower than the minimal version
    /// you requested.
    ///
    /// The advertised version is provided as argument.
    fn error(&mut self, _version: u32) {}
}

impl<F, I: Interface> GlobalImplementor<I> for F
where
    F: FnMut(NewProxy<I>) -> I,
{
    fn new_global(&mut self, global: NewProxy<I>) -> I {
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
/// # use wayland_client::{Display, NewProxy};
/// use wayland_client::protocol::{wl_output, wl_seat};
///
/// # fn main() {
/// # let (display, mut event_queue) = Display::connect_to_env().unwrap();
/// # let seat_implementor: fn(NewProxy<_>) -> _ = unimplemented!();
/// # let output_implementor: fn(NewProxy<_>) -> _ = unimplemented!();
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
            use $crate::protocol::wl_registry;
            use $crate::{GlobalEvent, NewProxy, Interface, GlobalImplementor};
            type Callback = Box<FnMut(u32, u32, wl_registry::WlRegistry)>;
            let mut callbacks: Vec<(&'static str, Callback)> = Vec::new();
            // Create the callback list
            $({
                let mut cb = { $callback };
                callbacks.push((
                    <$interface as Interface>::NAME,
                    Box::new(move |id, version, registry: wl_registry::WlRegistry| {
                        if version < $version {
                            GlobalImplementor::<$interface>::error(&mut cb, version);
                        } else {
                            registry.bind::<$interface, _>(
                                version,
                                id,
                                |newp| GlobalImplementor::<$interface>::new_global(&mut cb, newp)
                            )
                            .expect("wl_registry died unexpectedly");
                        }
                    }) as Box<_>
                ));
            })*

            // return the global closure
            move |event: GlobalEvent, registry: wl_registry::WlRegistry| {
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
