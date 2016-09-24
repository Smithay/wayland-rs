use EventQueueHandle;
use protocol::wl_registry::WlRegistry;

#[doc(hidden)]
pub trait EnvHandlerInner: Sized {
    fn create(&WlRegistry, &[(u32, String, u32)]) -> Option<Self>;
}

pub struct EnvHandler<H: EnvHandlerInner> {
    globals: Vec<(u32, String, u32)>,
    inner: Option<H>
}

impl<H: EnvHandlerInner> EnvHandler<H> {
    pub fn new() -> EnvHandler<H> {
        EnvHandler {
            globals: Vec::new(),
            inner: None
        }
    }

    pub fn ready(&self) -> bool {
        self.inner.is_some()
    }

    pub fn globals(&self) -> &[(u32, String, u32)] {
        &self.globals
    }

    fn try_make_ready(&mut self, registry: &WlRegistry) {
        if self.inner.is_some() { return; }
        self.inner = H::create(registry, &self.globals);
    }
}

impl<H: EnvHandlerInner> ::std::ops::Deref for EnvHandler<H> {
    type Target = H;
    fn deref(&self) -> &H {
        self.inner.as_ref().expect("Tried to get contents of a not-ready EnvHandler.")
    }
}

impl<H: EnvHandlerInner> ::protocol::wl_registry::Handler for EnvHandler<H> {
    fn global(&mut self, _: &mut EventQueueHandle, registry: &WlRegistry, name: u32, interface: String, version: u32) {
        self.globals.push((name, interface, version));
        self.try_make_ready(registry);
    }
    fn global_remove(&mut self, _: &mut EventQueueHandle, _: &WlRegistry, name: u32) {
        self.globals.retain(|&(i,_,_)| i != name)
    }
}

unsafe impl<H: EnvHandlerInner> ::Handler<WlRegistry> for EnvHandler<H> {
    unsafe fn message(&mut self, evq: &mut EventQueueHandle, proxy: &WlRegistry, opcode: u32, args: *const ::sys::wl_argument) -> Result<(),()> {
        <EnvHandler<H> as ::protocol::wl_registry::Handler>::__message(self, evq, proxy, opcode, args)
    }
}

#[macro_export]
macro_rules! wayland_env(
    ($name: ident) => {
        struct $name;
        impl $crate::env::EnvHandlerInner for $name {
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

        impl $crate::env::EnvHandlerInner for $name {
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
                        registry.bind::<$global_type>(version, id).expect("Registry cannot be destroyed!")
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
