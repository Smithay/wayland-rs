use {EventQueueHandle, StateToken};
use protocol::wl_registry::{Implementation, WlRegistry};

#[doc(hidden)]
pub trait EnvHandlerInner: Sized {
    fn create(&WlRegistry, &[(u32, String, u32)]) -> Option<Self>;
    fn clone_env(&self) -> Self;
}

/// Utility type to handle the registry and global objects
///
/// This struct provides you with a generic handler for the `wl_registry`
/// and the instanciation of global objects.
///
/// To use it, you need to declare your globals of interest using the
/// `wayland_env!(..)` macro. Then, insert this handler in your event loop and
/// register your registry to it.
///
/// Once this handler is fully initialized (and all globals have been
/// instantiated), it makes them usable via deref-ing towards the struct
/// previously declared by the `wayland_env!(...)` macro.
///
/// The `globals()` method also give you a list of all globals declared by
/// the server, had them been instantiated or not. It is perfectly safe
/// to instantiate again a global that have already been instantiated.
///
/// This list is updated whenever the server declares or removes a global object,
/// (as long as you don't change the handler associated to your registry).
///
/// If you want to manage all you globals manually, but still want to use
/// this utility to maintain the list of evailable globals, you can simply
/// create an empty env type using the macro, like this : `wayland_env!(WaylandEnv)`.
/// No global will be automatically instantiated for you, but you still can use
/// this `globals()` method.
///
/// ## example of use
///
/// ```ignore
/// // Declare your globals of interest using the macro.
/// // This creates a struct named WaylandEnv (but you can change it).
/// wayland_env!(WaylandEnv, compositor: wl_compositor::WlCompositor);
///
/// let registry = display.get_registry();
/// let env_token = EnvHandler::<WaylandEnv>::init(&mut event_queue, &registry);
/// // a roundtrip sync will dispatch all event declaring globals to the handler
/// eventqueue.sync_roundtrip().unwrap();
/// // then, we can access them via the state of the event queue:
/// let state = eventqueue.state();
/// let env = state.get(&env_token);
/// // We can now access the globals as fields of env.
/// // Note that is some globals were missing, this acces (via Deref)
/// // will panic!
/// let compositor = env.compositor;
/// ```
pub struct EnvHandler<H: EnvHandlerInner> {
    globals: Vec<(u32, String, u32)>,
    inner: Option<H>,
}

impl<H: EnvHandlerInner + 'static> EnvHandler<H> {
    /// Insert a new EnvHandler in this event queue and register the registry to it
    pub fn init(evqh: &mut EventQueueHandle, registry: &WlRegistry) -> StateToken<EnvHandler<H>> {
        let token = {
            let state = evqh.state();
            state.insert(EnvHandler {
                globals: Vec::new(),
                inner: None,
            })
        };
        evqh.register(registry, env_handler_impl(), token.clone());
        token
    }

    /// Insert a new EnvHandler in this event queue with a notify implementation
    ///
    /// Does the same as `EnvHandler::init(..)`, but you additionnaly supply an implementation
    /// struct that the EnvHandler will use to notify you of events:
    ///
    /// - events of creation/deletion of a global are forwarded
    /// - event of readiness of this EnvHandler (when all the necessary globals could be bound)
    pub fn init_with_notify<ID: 'static>(evqh: &mut EventQueueHandle, registry: &WlRegistry,
                                         notify: EnvNotify<ID>, idata: ID)
                                         -> StateToken<EnvHandler<H>> {
        let token = {
            let state = evqh.state();
            state.insert(EnvHandler {
                globals: Vec::new(),
                inner: None,
            })
        };
        evqh.register(
            registry,
            env_handler_impl_notify(),
            (token.clone(), (notify, idata)),
        );
        token
    }

    /// Is the handler ready
    ///
    /// Returns true if all required globals have been created.
    ///
    /// If this method returns false, trying to access a global
    /// field will panic.
    pub fn ready(&self) -> bool {
        self.inner.is_some()
    }

    /// List of advertised globals
    ///
    /// Returns a list of all globals that have been advertised by the server.
    ///
    /// The type format of each tuple is: `(global_id, interface_name, global_version)`.
    pub fn globals(&self) -> &[(u32, String, u32)] {
        &self.globals
    }

    /// Retrieve an owned copy of the environment
    ///
    /// This clones the inner env so that you can have access to the
    /// clobals without borrowing the event queue
    pub fn clone_inner(&self) -> Option<H> {
        self.inner.as_ref().map(|h| h.clone_env())
    }

    /// Returns true if the env became ready after this call
    fn try_make_ready(&mut self, registry: &WlRegistry) -> bool {
        if self.inner.is_some() {
            return false;
        }
        self.inner = H::create(registry, &self.globals);
        self.ready()
    }
}

impl<H: EnvHandlerInner> ::std::ops::Deref for EnvHandler<H> {
    type Target = H;
    fn deref(&self) -> &H {
        self.inner
            .as_ref()
            .expect("Tried to get contents of a not-ready EnvHandler.")
    }
}

fn env_handler_impl<H: EnvHandlerInner + 'static>() -> Implementation<StateToken<EnvHandler<H>>> {
    Implementation {
        global: |evqh, token, registry, name, interface, version| {
            let state = evqh.state();
            let me = state.get_mut(token);
            me.globals.push((name, interface, version));
            me.try_make_ready(registry);
        },
        global_remove: |evqh, token, _, name| {
            let state = evqh.state();
            let me = state.get_mut(token);
            me.globals.retain(|&(i, _, _)| i != name)
        },
    }
}

fn env_handler_impl_notify<H: EnvHandlerInner + 'static, ID: 'static>(
    )
    -> Implementation<(StateToken<EnvHandler<H>>, (EnvNotify<ID>, ID))>
{
    Implementation {
        global: |evqh, &mut (ref token, (ref implem, ref mut idata)), registry, name, interface, version| {
            (implem.new_global)(evqh, idata, registry, name, &interface, version);
            let became_ready = {
                let state = evqh.state();
                let me = state.get_mut(token);
                me.globals.push((name, interface, version));
                me.try_make_ready(registry)
            };
            if became_ready {
                (implem.ready)(evqh, idata, registry);
            }
        },
        global_remove: |evqh, &mut (ref token, (ref implem, ref mut idata)), registry, name| {
            (implem.del_global)(evqh, idata, registry, name);
            let state = evqh.state();
            let me = state.get_mut(token);
            me.globals.retain(|&(i, _, _)| i != name);
        },
    }
}

/// An implementation to receive globals notifications for the EnvHandler
///
/// You can provide this implementation to have the EnvHandler notify you
/// about new globals in addition to processing them.
///
/// See `EnvHandler::init_with_notify(..)`.
pub struct EnvNotify<ID> {
    /// A new global was advertized by the server
    ///
    /// Arguments are:
    ///
    /// - The `&mut EventQueueHandle`
    /// - A mutable reference to the implementation data you provided
    /// - A handle to the wayland registry
    /// - the id of this new global
    /// - the interface of this global
    /// - the version of this global
    pub new_global: fn(
     evqh: &mut EventQueueHandle,
     idata: &mut ID,
     registry: &WlRegistry,
     id: u32,
     interface: &str,
     version: u32,
    ),
    /// A global was removed by the server
    ///
    /// Arguments are:
    ///
    /// - The `&mut EventQueueHandle`
    /// - A mutable reference to the implementation data you provided
    /// - A handle to the wayland registry
    /// - the id of the removed global
    pub del_global: fn(evqh: &mut EventQueueHandle, idata: &mut ID, registry: &WlRegistry, id: u32),
    /// The EnvHandler is ready
    ///
    /// This is called once all necessary globals defined with the `wayland_env!()`
    /// macro were advertized and instanciated.
    ///
    /// Arguments are:
    ///
    /// - The `&mut EventQueueHandle`
    /// - A mutable reference to the implementation data you provided
    /// - A handle to the wayland registry
    pub ready: fn(evqh: &mut EventQueueHandle, idata: &mut ID, registry: &WlRegistry),
}

/// Create an environment handling struct
///
/// To be used in conjunction with the `EnvHandler` utility.
///
/// Declare the globals your application needs to use, like this, following the
/// general pattern: `$name : $type`:
///
/// ```ignore
/// use wayland_client::protocol::{Wl_compositor,wl_shell};
///
/// wayland_env!(WaylandEnv,
///     compositor: wl_compositor::WlCompositor,
///     shell : wl_shell::WlShell
/// );
/// ```
///
/// `$name` (`compositor` and `shell` in this example) are the name of the
/// fields that will contain the global objects one initialisation is done.
/// `$type` must be a wayland object type, implementing the `Proxy` trait.
///
/// If more than one field with a given type are provided, the handler will expect
/// the server to declare as many global objects of given type. If more globals of
/// a given type are declared by the server than in this macro, only the first `N`
/// will be bound in the environment struct.
///
/// This utility will interpret all globals declared in this macro as _necessary_, and
/// thus will not give you access to anything util they have all been declared by the compositor.
/// As such, only declare globals that your application cannot run without, like probably
/// `wl_compositor`, `wl_shm` or `wl_seat`. If there are globals that you can optionnaly
/// use, you'll have to instantiate them manually via `WlRegistry::bind(..)`.
#[macro_export]
macro_rules! wayland_env(
    (pub $name: ident) => {
        pub struct $name;
        impl $crate::EnvHandlerInner for $name {
            fn create(_registry: &$crate::protocol::wl_registry::WlRegistry, _globals: &[(u32, String, u32)]) -> Option<$name> {
                Some($name)
            }
            fn clone_env(&self) -> $name {
                $name
            }
        }
    };
    (pub $name: ident, $($global_name: ident : $global_type: path),+) => {
        pub struct $name {
            $(
                pub $global_name: $global_type
            ),+
        }
        wayland_env!(__impl $name, $($global_name : $global_type),+);
    };
    ($name: ident) => {
        struct $name;
        impl $crate::EnvHandlerInner for $name {
            fn create(_registry: &$crate::protocol::wl_registry::WlRegistry, _globals: &[(u32, String, u32)]) -> Option<$name> {
                Some($name)
            }
            fn clone_env(&self) -> $name {
                $name
            }
        }
    };
    ($name: ident, $($global_name: ident : $global_type: path),+) => {
        struct $name {
            $(
                pub $global_name: $global_type
            ),+
        }
        wayland_env!(__impl $name, $($global_name : $global_type),+);
    };
    (__impl $name: ident, $($global_name: ident : $global_type: path),+) => {
        impl $crate::EnvHandlerInner for $name {
            fn create(registry: &$crate::protocol::wl_registry::WlRegistry, globals: &[(u32, String, u32)]) -> Option<$name> {
                // hopefully llvm will optimize this
                let mut need = 0usize;
                $(
                    let _ = stringify!($global_name);
                    need += 1;
                )+
                if need > globals.len() { return None }

                let mut my_globals: Vec<(u32, &str, u32)> = globals.iter().map(|&(i,ref n,v)| (i,&n[..],v)).collect();

                $(
                    let $global_name = {
                        let iname = <$global_type as $crate::Proxy>::interface_name();
                        let index = my_globals.iter().position(|&(_,name,_)| name == iname);
                        match index {
                            None => return None,
                            Some(i) => my_globals.swap_remove(i)
                        }
                    };
                )*
                $(
                    let $global_name = {
                        let (id, _, v) = $global_name;
                        let version = ::std::cmp::min(v, <$global_type as $crate::Proxy>::supported_version());
                        registry.bind::<$global_type>(version, id)
                    };
                )+
                Some($name {
                    $(
                        $global_name: $global_name
                    ),+
                })
            }

            fn clone_env(&self) -> $name {
                $name {
                    $(
                        $global_name: $crate::Proxy::clone(&self.$global_name).unwrap()
                    ),+
                }
            }
        }
    };
);
