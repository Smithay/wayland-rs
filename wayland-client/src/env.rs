use EventQueueHandle;
use protocol::wl_registry::WlRegistry;

#[doc(hidden)]
pub trait EnvHandlerInner: Sized {
    fn create(&WlRegistry, &[(u32, String, u32)]) -> Option<Self>;
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
/// let env_id = eventqueue.add_handler(EnvHandler::<WaylandEnv>::new());
/// eventqueue.register::<_, EnvHandler<WaylandEnv>>(&registry, env_id);
/// // a roundtrip sync will dispatch all event declaring globals to the handler
/// eventqueue.sync_roundtrip().unwrap();
/// // then, we can access them via the state of the event queue:
/// let state = eventqueue.state();
/// let env = state.get_handler::<EnvHandler<WaylandEnv>>(env_id);
/// // We can now access the globals as fields of env.
/// // Note that is some globals were missing, this acces (via Deref)
/// // will panic!
/// let compositor = env.compositor;
/// ```
pub struct EnvHandler<H: EnvHandlerInner> {
    globals: Vec<(u32, String, u32)>,
    inner: Option<H>,
}

impl<H: EnvHandlerInner> EnvHandler<H> {
    /// Create a new EnvHandler
    ///
    /// This handler will need to be given to an event queue
    /// and a `WlRegistry` be registered to it.
    pub fn new() -> EnvHandler<H> {
        EnvHandler {
            globals: Vec::new(),
            inner: None,
        }
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

    fn try_make_ready(&mut self, registry: &WlRegistry) {
        if self.inner.is_some() {
            return;
        }
        self.inner = H::create(registry, &self.globals);
    }
}

impl<H: EnvHandlerInner> ::std::ops::Deref for EnvHandler<H> {
    type Target = H;
    fn deref(&self) -> &H {
        self.inner.as_ref().expect(
            "Tried to get contents of a not-ready EnvHandler.",
        )
    }
}

impl<H: EnvHandlerInner> ::protocol::wl_registry::Handler for EnvHandler<H> {
    fn global(&mut self, _: &mut EventQueueHandle, registry: &WlRegistry, name: u32, interface: String,
              version: u32) {
        self.globals.push((name, interface, version));
        self.try_make_ready(registry);
    }
    fn global_remove(&mut self, _: &mut EventQueueHandle, _: &WlRegistry, name: u32) {
        self.globals.retain(|&(i, _, _)| i != name)
    }
}

unsafe impl<H: EnvHandlerInner> ::Handler<WlRegistry> for EnvHandler<H> {
    unsafe fn message(&mut self, evq: &mut EventQueueHandle, proxy: &WlRegistry, opcode: u32,
                      args: *const ::sys::wl_argument)
                      -> Result<(), ()> {
        <EnvHandler<H> as ::protocol::wl_registry::Handler>::__message(self, evq, proxy, opcode, args)
    }
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
        }
    };
);
