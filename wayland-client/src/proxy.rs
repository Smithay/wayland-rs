use wayland_commons::{Implementation, Interface, MessageGroup};

#[cfg(feature = "native_lib")]
use wayland_sys::client::*;

use event_queue::QueueToken;

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
    #[cfg(feature = "native_lib")] is_wrapper: bool,
}

unsafe impl<I: Interface> Send for Proxy<I> {}
unsafe impl<I: Interface> Sync for Proxy<I> {}

impl<I: Interface> Proxy<I> {
    pub fn send(&self, msg: I::Requests) {
        #[cfg(not(feature = "native_lib"))]
        {
            if !self.internal.alive.load(Ordering::Acquire) {
                // don't send message to dead objects !
                return;
            }
            unimplemented!()
        }
        #[cfg(feature = "native_lib")]
        {
            if let Some(ref internal) = self.internal {
                // object is managed
                if !internal.alive.load(Ordering::Acquire) {
                    // don't send message to dead objects !
                    return;
                }
            }
            msg.as_raw_c_in(|opcode, args| unsafe {
                ffi_dispatch!(
                    WAYLAND_CLIENT_HANDLE,
                    wl_proxy_marshal_array,
                    self.ptr,
                    opcode,
                    args.as_ptr() as *mut _
                );
            });
        }
    }

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

    pub fn set_user_data(&self, ptr: *mut ()) {
        #[cfg(not(feature = "native_lib"))]
        {
            self.internal.user_data.store(ptr, Ordering::Release);
        }
        #[cfg(feature = "native_lib")]
        {
            if let Some(ref inner) = self.internal {
                inner.user_data.store(ptr, Ordering::Release);
            }
        }
    }

    pub fn get_user_data(&self) -> *mut () {
        #[cfg(not(feature = "native_lib"))]
        {
            self.internal.user_data.load(Ordering::Acquire)
        }
        #[cfg(feature = "native_lib")]
        {
            if let Some(ref inner) = self.internal {
                inner.user_data.load(Ordering::Acquire)
            } else {
                ::std::ptr::null_mut()
            }
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
            #[cfg(feature = "native_lib")]
            is_wrapper: self.is_wrapper,
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
            is_wrapper: false,
        }
    }

    #[doc(hidden)]
    #[cfg(feature = "native_lib")]
    pub unsafe fn new_null() -> Proxy<I> {
        Proxy {
            _i: ::std::marker::PhantomData,
            internal: None,
            ptr: ::std::ptr::null_mut(),
            is_wrapper: false,
        }
    }

    #[cfg(feature = "native_lib")]
    pub fn make_wrapper(&self, queue: &QueueToken) -> Result<Proxy<I>, ()> {
        if !self.is_external() && !self.is_alive() {
            return Err(());
        }

        let wrapper_ptr;
        unsafe {
            wrapper_ptr = ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_create_wrapper, self.ptr);
            queue.assign_proxy(wrapper_ptr);
        }

        Ok(Proxy {
            _i: ::std::marker::PhantomData,
            internal: self.internal.clone(),
            ptr: wrapper_ptr,
            is_wrapper: true,
        })
    }

    pub fn child<C: Interface>(&self) -> NewProxy<C> {
        #[cfg(not(feature = "native_lib"))]
        {
            unimplemented!()
        }
        #[cfg(feature = "native_lib")]
        {
            let ptr = unsafe {
                ffi_dispatch!(
                    WAYLAND_CLIENT_HANDLE,
                    wl_proxy_create,
                    self.ptr,
                    C::c_interface()
                )
            };
            NewProxy {
                _i: ::std::marker::PhantomData,
                ptr: ptr,
            }
        }
    }
}

#[cfg(feature = "native_lib")]
impl Proxy<::protocol::wl_display::WlDisplay> {
    pub(crate) fn from_display(d: *mut wl_display) -> Proxy<::protocol::wl_display::WlDisplay> {
        Proxy {
            _i: ::std::marker::PhantomData,
            internal: None,
            ptr: d as *mut wl_proxy,
            is_wrapper: false,
        }
    }
}

impl<I: Interface> Drop for Proxy<I> {
    fn drop(&mut self) {
        #[cfg(feature = "native_lib")]
        {
            if self.is_wrapper {
                unsafe {
                    ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_wrapper_destroy, self.ptr);
                }
            }
        }
    }
}

pub struct NewProxy<I: Interface> {
    _i: ::std::marker::PhantomData<*const I>,
    #[cfg(feature = "native_lib")] ptr: *mut wl_proxy,
}

impl<I: Interface + 'static> NewProxy<I> {
    /// Implement this proxy using given function and implementation data.
    pub fn implement<ID: 'static + Send>(
        self,
        idata: ID,
        implementation: Implementation<Proxy<I>, I::Events, ID>,
    ) -> Proxy<I> {
        unsafe { self.implement_inner(idata, implementation) }
    }

    /// Implement this proxy using given function and implementation data.
    ///
    /// This method allows the implementation data to not be `Send`, but requires for
    /// safety that you provide a token to the event queue this proxy will be implemented
    /// on. This method will then ensure that this proxy is registered on this event queue,
    /// so that it cannot be dispatched from an other thread.
    ///
    /// **Unsafety:**
    ///
    /// This call can be racy if the proxy is not already registered on this event queue and its
    /// old queue is being dispatched from an other thread.
    ///
    /// To ensure safety, see `Proxy::make_wrapper`.
    pub unsafe fn implement_nonsend<ID: 'static>(
        self,
        idata: ID,
        implementation: Implementation<Proxy<I>, I::Events, ID>,
        queue: &QueueToken,
    ) -> Proxy<I> {
        #[cfg(not(feature = "native_lib"))]
        {}
        #[cfg(feature = "native_lib")]
        {
            queue.assign_proxy(self.c_ptr());
            self.implement_inner(idata, implementation)
        }
    }

    unsafe fn implement_inner<ID: 'static>(
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

            ffi_dispatch!(
                WAYLAND_CLIENT_HANDLE,
                wl_proxy_add_dispatcher,
                self.ptr,
                self::native_machinery::proxy_dispatcher::<I, ID>,
                &::wayland_sys::RUST_MANAGED as *const _ as *const _,
                Box::into_raw(new_user_data) as *mut _
            );

            Proxy {
                _i: ::std::marker::PhantomData,
                internal: Some(internal),
                ptr: self.ptr,
                is_wrapper: false,
            }
        }
    }

    #[cfg(feature = "native_lib")]
    pub fn c_ptr(&self) -> *mut wl_proxy {
        self.ptr
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

    use super::Proxy;

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
        _msg: *const wl_message,
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
