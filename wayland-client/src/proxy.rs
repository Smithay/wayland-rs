use wayland_commons::{Implementation, Interface, MessageGroup};

#[cfg(feature = "native_lib")]
use wayland_sys::client::*;

use event_queue::QueueToken;

use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};
use std::sync::Arc;

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

/// An handle to a wayland proxy
///
/// This represents a wayland object instanciated in your client
/// session. Several handles to the same object can exist at a given
/// time, and cloning them won't create a new protocol object, only
/// clone the handle. The lifetime of the protocol object is **not**
/// tied to the lifetime of these handles, but rather to sending or
/// receiving destroying messages.
///
/// These handles are notably used to send requests to the server. To do
/// you need to import the associated `RequestsTrait` trait from the module
/// of this interface.
pub struct Proxy<I: Interface> {
    _i: ::std::marker::PhantomData<*const I>,
    #[cfg(not(feature = "native_lib"))]
    internal: Arc<ProxyInternal>,
    #[cfg(feature = "native_lib")]
    internal: Option<Arc<ProxyInternal>>,
    #[cfg(feature = "native_lib")]
    ptr: *mut wl_proxy,
    #[cfg(feature = "native_lib")]
    is_wrapper: bool,
}

unsafe impl<I: Interface> Send for Proxy<I> {}
unsafe impl<I: Interface> Sync for Proxy<I> {}

impl <I: Interface> PartialEq for Proxy<I> {
    fn eq(&self, other: &Proxy<I>) -> bool {
        self.equals(other)
    }
}

impl <I: Interface> Eq for Proxy<I> {}

impl<I: Interface> Proxy<I> {
    /// Send a request through this object
    ///
    /// This is the generic method to send requests.
    ///
    /// Several requests require the creation of new objects using
    /// the `child()` method, which if done wrong can cause protocol
    /// errors (in which case the server will terminate your connexion).
    /// Thus unless your know exactly what you are doing, you should use
    /// the helper methods provided by the various `RequestsTrait` for
    /// each interface, which handle this correctly for you.
    pub fn send(&self, msg: I::Request) {
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
            let destructor = msg.is_destructor();
            msg.as_raw_c_in(|opcode, args| unsafe {
                ffi_dispatch!(
                    WAYLAND_CLIENT_HANDLE,
                    wl_proxy_marshal_array,
                    self.ptr,
                    opcode,
                    args.as_ptr() as *mut _
                );
            });

            if destructor {
                // we need to destroy the proxy now
                if let Some(ref internal) = self.internal {
                    internal.alive.store(false, Ordering::Release);
                }
                unsafe {
                    ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_destroy, self.ptr);
                }
            }
        }
    }

    /// Check if the object associated with this proxy is still alive
    ///
    /// Will return `false` if either:
    ///
    /// - The object has been destroyed
    /// - The object is not managed by this library (see the `from_c_ptr` method)
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

    /// Retrieve the interface version of this wayland object instance
    ///
    /// Returns 0 on dead objects
    pub fn version(&self) -> u32 {
        if !self.is_alive() {
            return 0;
        }

        #[cfg(not(feature = "native_lib"))]
        {
            unimplemented!();
        }
        #[cfg(feature = "native_lib")]
        {
            unsafe { ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_get_version, self.ptr) as u32 }
        }
    }

    /// Associate an arbitrary payload to this object
    ///
    /// The pointer you associate here can be retrieved from any
    /// other proxy to the same wayland object.
    ///
    /// Setting or getting user data is done as an atomic operation.
    /// You are responsible for the correct initialization of this
    /// pointer, synchronisation of access, and destruction of the
    /// contents at the appropriate time.
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

    /// Retrieve the arbitrary payload associated to this object
    ///
    /// See `set_user_data` for explanations.
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
    /// Check whether this proxy is managed by the library or not
    ///
    /// See `from_c_ptr` for details.
    pub fn is_external(&self) -> bool {
        self.internal.is_none()
    }

    /// Check if the other proxy refers to the same underlying wayland object
    pub fn equals(&self, other: &Proxy<I>) -> bool {
        #[cfg(not(feature = "native_lib"))]
        {
            unimplemented!()
        }
        #[cfg(feature = "native_lib")]
        {
            match (&self.internal, &other.internal) {
                (&Some(ref my_inner), &Some(ref other_inner)) => Arc::ptr_eq(my_inner, other_inner),
                (&None, &None) => self.ptr == other.ptr,
                _ => false,
            }
        }
    }

    #[cfg(feature = "native_lib")]
    /// Get a raw pointer to the underlying wayland object
    ///
    /// Retrieve a pointer to the object from the `libwayland-client.so` library.
    /// You will mostly need it to interface with C libraries needing access
    /// to wayland objects (to initialize an opengl context for example).
    pub fn c_ptr(&self) -> *mut wl_proxy {
        self.ptr
    }

    #[cfg(feature = "native_lib")]
    /// Create a `Proxy` instance from a C pointer
    ///
    /// Create a `Proxy` from a raw pointer to a wayland object from the
    /// C library.
    ///
    /// If the pointer was previously obtained by the `c_ptr()` method, this
    /// constructs a new proxy for the same object just like the `clone()`
    /// method would have.
    ///
    /// If the object was created by some other C library you are interfacing
    /// with, it will be created in an "unmanaged" state: wayland-client will
    /// treat it as foreign, and as such most of the safeties will be absent.
    /// Notably the lifetime of the object can't be tracked, so the `alive()`
    /// method will always return `false` and you are responsible of not using
    /// an object past its destruction (as this would cause a protocol error).
    /// You will also be unable to associate any user data pointer to this object.
    ///
    /// In order to handle protocol races, invoking it with a NULL pointer will
    /// create an already-dead object.
    pub unsafe fn from_c_ptr(ptr: *mut wl_proxy) -> Self {
        use wayland_sys::client::*;

        if ptr.is_null() {
            return Proxy {
                _i: ::std::marker::PhantomData,
                internal: Some(Arc::new(ProxyInternal {
                    alive: AtomicBool::new(false),
                    user_data: AtomicPtr::new(::std::ptr::null_mut()),
                })),
                ptr: ptr,
                is_wrapper: false,
            };
        }

        let is_managed = {
            ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_get_listener, ptr)
                == &::wayland_sys::RUST_MANAGED as *const u8 as *const _
        };
        let internal = if is_managed {
            let user_data = ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_get_user_data, ptr)
                as *mut self::native_machinery::ProxyUserData<I>;
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
    /// Create a wrapper for this object for queue management
    ///
    /// As assigning a proxy to an event queue can be a racy operation
    /// in contextes involving multiple thread, this provides a facility
    /// to do this safely.
    ///
    /// The wrapper object created behaves like a regular `Proxy`, except that
    /// all objects created as the result of its requests will be assigned to
    /// the queue associated to the provided token, rather than the queue of
    /// their parent. This does not change the queue of the proxy itself.
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

    /// Create a new child object
    ///
    /// This creates a new wayland object, considered as a
    /// child of this object. It will notably inherit its interface
    /// version.
    ///
    /// The created object should immediatly be implemented and sent
    /// in a request to the server, to keep the object list properly
    /// synchronized. Failure to do so will likely cause a protocol
    /// error.
    pub fn child<C: Interface>(&self) -> NewProxy<C> {
        #[cfg(not(feature = "native_lib"))]
        {
            unimplemented!()
        }
        #[cfg(feature = "native_lib")]
        {
            let ptr =
                unsafe { ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_create, self.ptr, C::c_interface()) };
            NewProxy {
                _i: ::std::marker::PhantomData,
                ptr: ptr,
            }
        }
    }

    /// Check whether this proxy has been implemented with given type
    ///
    /// Always returns false if the proxy is no longer alive
    pub fn is_implemented_with<Impl>(&self) -> bool
    where
        Impl: Implementation<Proxy<I>, I::Event> + 'static,
    {
        if !self.is_alive() {
            return false;
        }
        #[cfg(not(feature = "native_lib"))]
        {
            unimplemented!();
        }
        #[cfg(feature = "native_lib")]
        {
            let user_data = unsafe {
                let ptr = ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_get_user_data, self.ptr)
                    as *mut self::native_machinery::ProxyUserData<I>;
                &*ptr
            };
            user_data.is_impl::<Impl>()
        }
    }
}

impl<I: Interface> Clone for Proxy<I> {
    fn clone(&self) -> Proxy<I> {
        Proxy {
            _i: ::std::marker::PhantomData,
            internal: self.internal.clone(),
            #[cfg(feature = "native_lib")]
            ptr: self.ptr,
            #[cfg(feature = "native_lib")]
            is_wrapper: self.is_wrapper,
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

    pub(crate) fn from_display_wrapper(d: *mut wl_proxy) -> Proxy<::protocol::wl_display::WlDisplay> {
        Proxy {
            _i: ::std::marker::PhantomData,
            internal: None,
            ptr: d as *mut wl_proxy,
            is_wrapper: true,
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

/// A newly-created proxy that needs implementation
///
/// Whenever a new wayland object is created, you will
/// receive it as a `NewProxy`. You then have to provide an
/// implementation for it, in order to process the incoming
/// events it may receive. Once this done you will be able
/// to use it as a regular `Proxy`.
///
/// Implementations are structs implementing the appropriate
/// variant of the `Implementation` trait. They can also be
/// closures.
pub struct NewProxy<I: Interface> {
    _i: ::std::marker::PhantomData<*const I>,
    #[cfg(feature = "native_lib")]
    ptr: *mut wl_proxy,
}

impl<I: Interface + 'static> NewProxy<I> {
    /// Implement this proxy using given function and implementation data.
    pub fn implement<Impl>(self, implementation: Impl) -> Proxy<I>
    where
        Impl: Implementation<Proxy<I>, I::Event> + Send + 'static,
    {
        unsafe { self.implement_inner(implementation) }
    }

    /// Implement this proxy using given function and implementation data.
    ///
    /// This method allows the implementation to not be `Send`, but requires for
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
    pub unsafe fn implement_nonsend<Impl>(self, implementation: Impl, queue: &QueueToken) -> Proxy<I>
    where
        Impl: Implementation<Proxy<I>, I::Event> + 'static,
    {
        #[cfg(not(feature = "native_lib"))]
        {}
        #[cfg(feature = "native_lib")]
        {
            queue.assign_proxy(self.c_ptr());
            self.implement_inner(implementation)
        }
    }

    unsafe fn implement_inner<Impl>(self, implementation: Impl) -> Proxy<I>
    where
        Impl: Implementation<Proxy<I>, I::Event> + 'static,
    {
        #[cfg(not(feature = "native_lib"))]
        {
            unimplemented!()
        }
        #[cfg(feature = "native_lib")]
        {
            use wayland_sys::client::*;

            let new_user_data = Box::new(self::native_machinery::ProxyUserData::new(implementation));
            let internal = new_user_data.internal.clone();

            ffi_dispatch!(
                WAYLAND_CLIENT_HANDLE,
                wl_proxy_add_dispatcher,
                self.ptr,
                self::native_machinery::proxy_dispatcher::<I>,
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
    /// Get a raw pointer to the underlying wayland object
    ///
    /// Retrieve a pointer to the object from the `libwayland-client.so` library.
    /// You will mostly need it to interface with C libraries needing access
    /// to wayland objects (to initialize an opengl context for example).
    ///
    /// Use this if you need to pass an unimplemented object to the C library
    /// you are interfacing with.
    pub fn c_ptr(&self) -> *mut wl_proxy {
        self.ptr
    }

    #[cfg(feature = "native_lib")]
    /// Create a `NewProxy` instance from a C pointer.
    ///
    /// By doing so, you assert that this wayland object was newly created and
    /// can be safely implemented. As implementing it will overwrite any previously
    /// associated data or implementation, this can cause weird errors akin to
    /// memory corruption if it was not the case.
    pub unsafe fn from_c_ptr(ptr: *mut wl_proxy) -> Self {
        NewProxy {
            _i: ::std::marker::PhantomData,
            ptr: ptr,
        }
    }
}

#[cfg(feature = "native_lib")]
mod native_machinery {
    use wayland_sys::client::*;
    use wayland_sys::common::*;

    use std::os::raw::{c_int, c_void};
    use std::sync::atomic::Ordering;
    use std::sync::Arc;

    use super::Proxy;

    use wayland_commons::{Implementation, Interface, MessageGroup};

    pub(crate) struct ProxyUserData<I: Interface> {
        pub(crate) internal: Arc<super::ProxyInternal>,
        implem: Option<Box<Implementation<Proxy<I>, I::Event>>>,
    }

    impl<I: Interface> ProxyUserData<I> {
        pub(crate) fn new<Impl>(implem: Impl) -> ProxyUserData<I>
        where
            Impl: Implementation<Proxy<I>, I::Event> + 'static,
        {
            ProxyUserData {
                internal: Arc::new(super::ProxyInternal::new()),
                implem: Some(Box::new(implem)),
            }
        }

        pub(crate) fn is_impl<Impl>(&self) -> bool
        where
            Impl: Implementation<Proxy<I>, I::Event> + 'static,
        {
            self.implem
                .as_ref()
                .map(|implem| implem.is::<Impl>())
                .unwrap_or(false)
        }
    }

    pub(crate) unsafe extern "C" fn proxy_dispatcher<I: Interface>(
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
            let msg = I::Event::from_raw_c(proxy as *mut _, opcode, args)?;
            let must_destroy = msg.is_destructor();
            // create the proxy object
            let proxy_obj = super::Proxy::<I>::from_c_ptr(proxy);
            // retrieve the impl
            let user_data = ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_get_user_data, proxy);
            {
                let user_data = &mut *(user_data as *mut ProxyUserData<I>);
                let implem = user_data.implem.as_mut().unwrap();
                if must_destroy {
                    user_data.internal.alive.store(false, Ordering::Release);
                }
                // call the impl
                implem.receive(msg, proxy_obj);
            }
            if must_destroy {
                // final cleanup
                let _ = Box::from_raw(user_data as *mut ProxyUserData<I>);
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
