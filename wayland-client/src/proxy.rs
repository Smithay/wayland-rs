use wayland_commons::{Implementation, Interface};

#[cfg(feature = "native_lib")]
use wayland_sys::client::wl_proxy;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};

pub(crate) struct ProxyInternal {
    alive: AtomicBool,
    user_data: AtomicPtr<()>,
}

impl ProxyInternal {
    fn new() -> ProxyInternal {
        ProxyInternal {
            alive: AtomicBool::new(true),
            user_data: AtomicPtr::new(::std::ptr::null_mut()),
        }
    }
}

pub struct Proxy<I: Interface> {
    _i: ::std::marker::PhantomData<*const I>,
    #[cfg(not(feature = "native_lib"))] internal: Arc<ProxyInternal>,
    #[cfg(feature = "native_lib")] internal: Option<Arc<ProxyInternal>>,
    #[cfg(feature = "native_lib")] ptr: *mut wl_proxy,
}

impl<I: Interface> Proxy<I> {
    // returns false is external
    pub fn is_alive(&self) -> bool {
        #[cfg(not(feature = "native_lib"))]
        {
            self.internal.alive.load(Ordering::Acquire)
        }
        #[cfg(feature = "native_lib")]
        {
            self.internal
                .as_ref()
                .map(|i| i.alive.load(Ordering::Acquire))
                .unwrap_or(false)
        }
    }

    #[cfg(feature = "native_lib")]
    pub fn is_external(&self) -> bool {
        self.internal.is_none()
    }

    pub fn clone(&self) -> Proxy<I> {
        Proxy {
            _i: ::std::marker::PhantomData,
            internal: self.internal.clone(),
            #[cfg(feature = "native_lib")]
            ptr: self.ptr,
        }
    }

    #[cfg(feature = "native_lib")]
    pub fn c_ptr(&self) -> *mut wl_proxy {
        self.ptr
    }

    #[cfg(feature = "native_lib")]
    pub unsafe fn from_c_ptr(ptr: *mut wl_proxy) -> Self {
        use wayland_sys::client::*;

        let is_managed = {
            ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_get_listener, ptr)
                == &::wayland_sys::RUST_MANAGED as *const u8 as *const _
        };
        let internal = if is_managed {
            let user_data = ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_get_user_data, ptr)
                as *mut self::native_machinery::ProxyUserData;
            Some((*user_data).internal.clone())
        } else {
            None
        };
        Proxy {
            _i: ::std::marker::PhantomData,
            internal: internal,
            ptr: ptr,
        }
    }
}

pub struct NewProxy<I: Interface> {
    _i: ::std::marker::PhantomData<*const I>,
    #[cfg(feature = "native_lib")] ptr: *mut wl_proxy,
}

impl<I: Interface + 'static> NewProxy<I> {
    pub fn implement<ID: 'static>(
        self,
        idata: ID,
        implementation: Implementation<Proxy<I>, I::Events, ID>,
    ) -> Proxy<I> {
        #[cfg(not(feature = "native_lib"))]
        {
            unimplemented!()
        }
        #[cfg(feature = "native_lib")]
        {
            use wayland_sys::client::*;

            let new_user_data = Box::new(self::native_machinery::ProxyUserData::new(
                idata,
                implementation,
            ));
            let internal = new_user_data.internal.clone();

            unsafe {
                ffi_dispatch!(
                    WAYLAND_CLIENT_HANDLE,
                    wl_proxy_add_dispatcher,
                    self.ptr,
                    self::native_machinery::proxy_dispatcher::<I, ID>,
                    &::wayland_sys::RUST_MANAGED as *const _ as *const _,
                    Box::into_raw(new_user_data) as *mut _
                );
            }

            Proxy {
                _i: ::std::marker::PhantomData,
                internal: Some(internal),
                ptr: self.ptr,
            }
        }
    }

    #[cfg(feature = "native_lib")]
    pub unsafe fn from_c_ptr(ptr: *mut wl_proxy) -> Self {
        NewProxy {
            _i: ::std::marker::PhantomData,
            ptr: ptr,
        }
    }
}

#[cfg(feature = "native_lib")]
mod native_machinery {
    use wayland_sys::common::*;
    use wayland_sys::client::*;

    use std::any::Any;
    use std::sync::Arc;
    use std::sync::atomic::Ordering;
    use std::os::raw::{c_int, c_void};

    use super::{NewProxy, Proxy};

    use wayland_commons::{Implementation, Interface, MessageGroup};

    pub(crate) struct ProxyUserData {
        pub(crate) internal: Arc<super::ProxyInternal>,
        implem: Option<Box<Any>>,
    }

    impl ProxyUserData {
        pub(crate) fn new<I: Interface, ID: 'static>(
            idata: ID,
            implem: Implementation<Proxy<I>, I::Events, ID>,
        ) -> ProxyUserData {
            ProxyUserData {
                internal: Arc::new(super::ProxyInternal::new()),
                implem: Some(Box::new((implem, idata)) as Box<Any>),
            }
        }
    }

    pub(crate) unsafe extern "C" fn proxy_dispatcher<I: Interface, ID: 'static>(
        _implem: *const c_void,
        proxy: *mut c_void,
        opcode: u32,
        msg: *const wl_message,
        args: *const wl_argument,
    ) -> c_int
    where
        I: Interface,
    {
        let proxy = proxy as *mut wl_proxy;

        // We don't need to worry about panic-safeness, because if there is a panic,
        // we'll abort the process, so no access to corrupted data is possible.
        let ret = ::std::panic::catch_unwind(move || {
            // parse the message:
            let msg = I::Events::from_raw_c(proxy as *mut _, opcode, args)?;
            let must_destroy = msg.is_destructor();
            // create the proxy object
            let proxy_obj = super::Proxy::<I>::from_c_ptr(proxy);
            // retrieve the impl
            let user_data = ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_get_user_data, proxy);
            {
                let user_data = &mut *(user_data as *mut ProxyUserData);
                let implem = user_data
                    .implem
                    .as_mut()
                    .unwrap()
                    .downcast_mut::<(Implementation<Proxy<I>, I::Events, ID>, ID)>()
                    .unwrap();
                let &mut (ref implem_func, ref mut idata) = implem;
                if must_destroy {
                    user_data.internal.alive.store(false, Ordering::Release);
                }
                // call the impl
                implem_func(proxy_obj, msg, idata);
            }
            if must_destroy {
                // final cleanup
                let _ = Box::from_raw(user_data as *mut ProxyUserData);
                ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_destroy, proxy);
            }
            Ok(())
        });
        // check the return status
        match ret {
            Ok(Ok(())) => return 0,
            Ok(Err(())) => {
                eprintln!(
                    "[wayland-client error] Attempted to dispatch unknown opcode {} for {}, aborting.",
                    opcode,
                    I::NAME
                );
                ::libc::abort();
            }
            Err(_) => {
                eprintln!("[wayland-client error] A handler for {} panicked.", I::NAME);
                ::libc::abort()
            }
        }
    }
}
