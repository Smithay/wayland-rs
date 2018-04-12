use wayland_commons::{Implementation, Interface, MessageGroup};

use {Client, LoopToken};

#[cfg(feature = "native_lib")]
use wayland_sys::server::*;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};

pub(crate) struct ResourceInternal {
    alive: AtomicBool,
    user_data: AtomicPtr<()>,
}

impl ResourceInternal {
    fn new() -> ResourceInternal {
        ResourceInternal {
            alive: AtomicBool::new(true),
            user_data: AtomicPtr::new(::std::ptr::null_mut()),
        }
    }
}

/// An handle to a wayland resource
///
/// This represents a wayland object instanciated in a client
/// session. Several handles to the same object can exist at a given
/// time, and cloning them won't create a new protocol object, only
/// clone the handle. The lifetime of the protocol object is **not**
/// tied to the lifetime of these handles, but rather to sending or
/// receiving destroying messages.
///
/// These handles are notably used to send events to the associated client,
/// via the `send` method.
pub struct Resource<I: Interface> {
    _i: ::std::marker::PhantomData<*const I>,
    #[cfg(not(feature = "native_lib"))]
    internal: Arc<ResourceInternal>,
    #[cfg(feature = "native_lib")]
    internal: Option<Arc<ResourceInternal>>,
    #[cfg(feature = "native_lib")]
    ptr: *mut wl_resource,
}

impl<I: Interface> Resource<I> {
    /// Send an event through this object
    ///
    /// The event will be send to the client associated to this
    /// object.
    pub fn send(&self, msg: I::Event) {
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
                    WAYLAND_SERVER_HANDLE,
                    wl_resource_post_event_array,
                    self.ptr,
                    opcode,
                    args.as_ptr() as *mut _
                );
            });
        }
    }

    /// Check if the object associated with this resource is still alive
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
            unsafe { ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_resource_get_version, self.ptr) as u32 }
        }
    }

    #[cfg(feature = "native_lib")]
    /// Check whether this resource is managed by the library or not
    ///
    /// See `from_c_ptr` for details.
    pub fn is_external(&self) -> bool {
        self.internal.is_none()
    }

    /// Clone this resource
    ///
    /// It only creates a new handle to the same wayland
    /// object, and does not create a new one.
    pub fn clone(&self) -> Resource<I> {
        Resource {
            _i: ::std::marker::PhantomData,
            internal: self.internal.clone(),
            #[cfg(feature = "native_lib")]
            ptr: self.ptr,
        }
    }

    /// Posts a protocol error to this resource
    ///
    /// The error code can be obtained from the various `Error` enums of the protocols.
    ///
    /// An error is fatal to the client that caused it.
    pub fn post_error(&self, error_code: u32, msg: String) {
        #[cfg(not(feature = "native_lib"))]
        {
            unimplemented!();
        }
        #[cfg(feature = "native_lib")]
        {
            // If `str` contains an interior null, the actual transmitted message will
            // be truncated at this point.
            unsafe {
                let cstring = ::std::ffi::CString::from_vec_unchecked(msg.into());
                ffi_dispatch!(
                    WAYLAND_SERVER_HANDLE,
                    wl_resource_post_error,
                    self.ptr,
                    error_code,
                    cstring.as_ptr()
                )
            }
        }
    }

    /// Associate an arbitrary payload to this object
    ///
    /// The pointer you associate here can be retrieved from any
    /// other resource handle to the same wayland object.
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

    /// Retrieve an handle to the client associated with this resource
    ///
    /// Returns `None` if the resource is no longer alive.
    pub fn client(&self) -> Option<Client> {
        if self.is_alive() {
            #[cfg(not(feature = "native_lib"))]
            {
                unimplemented!()
            }
            #[cfg(feature = "native_lib")]
            {
                unsafe {
                    let client_ptr = ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_resource_get_client, self.ptr);
                    Some(Client::from_ptr(client_ptr))
                }
            }
        } else {
            None
        }
    }

    #[cfg(feature = "native_lib")]
    /// Get a raw pointer to the underlying wayland object
    ///
    /// Retrieve a pointer to the object from the `libwayland-server.so` library.
    /// You will mostly need it to interface with C libraries needing access
    /// to wayland objects (to initialize an opengl context for example).
    pub fn c_ptr(&self) -> *mut wl_resource {
        self.ptr
    }

    #[cfg(feature = "native_lib")]
    /// Create a `Resource` instance from a C pointer
    ///
    /// Create a `Resource` from a raw pointer to a wayland object from the
    /// C library.
    ///
    /// If the pointer was previously obtained by the `c_ptr()` method, this
    /// constructs a new resource handle for the same object just like the
    /// `clone()` method would have.
    ///
    /// If the object was created by some other C library you are interfacing
    /// with, it will be created in an "unmanaged" state: wayland-server will
    /// treat it as foreign, and as such most of the safeties will be absent.
    /// Notably the lifetime of the object can't be tracked, so the `alive()`
    /// method will always return `false` and you are responsible of not using
    /// an object past its destruction (as this would cause a protocol error).
    /// You will also be unable to associate any user data pointer to this object.
    pub unsafe fn from_c_ptr(ptr: *mut wl_resource) -> Self {
        let is_managed = {
            ffi_dispatch!(
                WAYLAND_SERVER_HANDLE,
                wl_resource_instance_of,
                ptr,
                I::c_interface(),
                &::wayland_sys::RUST_MANAGED as *const u8 as *const _
            ) != 0
        };
        let internal = if is_managed {
            let user_data = ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_resource_get_user_data, ptr)
                as *mut self::native_machinery::ResourceUserData<I>;
            Some((*user_data).internal.clone())
        } else {
            None
        };
        Resource {
            _i: ::std::marker::PhantomData,
            internal: internal,
            ptr: ptr,
        }
    }

    /// Check whether this resource has been implemented with given type
    ///
    /// Always returns false if the resource is no longer alive
    pub fn is_implemented_with<Impl>(&self) -> bool
    where
        Impl: Implementation<Resource<I>, I::Request> + 'static,
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
                let ptr = ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_resource_get_user_data, self.ptr)
                    as *mut self::native_machinery::ResourceUserData<I>;
                &*ptr
            };
            user_data.is_impl::<Impl>()
        }
    }
}

/// A newly-created resource that needs implementation
///
/// Whenever a new wayland object is created, you will
/// receive it as a `NewResource`. You then have to provide an
/// implementation for it, in order to process the incoming
/// events it may receive. Once this done you will be able
/// to use it as a regular `Resource`.
///
/// Implementations are structs implementing the appropriate
/// variant of the `Implementation` trait. They can also be
/// closures.
pub struct NewResource<I: Interface> {
    _i: ::std::marker::PhantomData<*const I>,
    #[cfg(feature = "native_lib")]
    ptr: *mut wl_resource,
}

impl<I: Interface + 'static> NewResource<I> {
    /// Implement this resource using given function, destructor, and implementation data.
    pub fn implement<Impl, Dest>(self, implementation: Impl, destructor: Option<Dest>) -> Resource<I>
    where
        Impl: Implementation<Resource<I>, I::Request> + Send + 'static,
        Dest: FnMut(Resource<I>, Box<Implementation<Resource<I>, I::Request>>) + Send + 'static,
    {
        unsafe { self.implement_inner(implementation, destructor) }
    }

    /// Implement this resource using given function and implementation data.
    ///
    /// This method allows the implementation data to not be `Send`, but requires for
    /// safety that you provide a token to the event loop owning the proxy. As the token
    /// is not `Send`, this ensures you are implementing the resource from the same thread
    /// as the event loop runs on.
    ///
    /// ** Unsafety **
    ///
    /// This function is unsafe if you create several wayland event loops and do not
    /// provide a token to the right one.
    pub unsafe fn implement_nonsend<Impl, Dest>(
        self,
        implementation: Impl,
        destructor: Option<Dest>,
        token: &LoopToken,
    ) -> Resource<I>
    where
        Impl: Implementation<Resource<I>, I::Request> + 'static,
        Dest: FnMut(Resource<I>, Box<Implementation<Resource<I>, I::Request>>) + 'static,
    {
        let _ = token;
        self.implement_inner(implementation, destructor)
    }

    unsafe fn implement_inner<Impl, Dest>(self, implementation: Impl, destructor: Option<Dest>) -> Resource<I>
    where
        Impl: Implementation<Resource<I>, I::Request> + 'static,
        Dest: FnMut(Resource<I>, Box<Implementation<Resource<I>, I::Request>>) + 'static,
    {
        #[cfg(not(feature = "native_lib"))]
        {
            unimplemented!()
        }
        #[cfg(feature = "native_lib")]
        {
            let new_user_data = Box::new(self::native_machinery::ResourceUserData::new(
                implementation,
                destructor,
            ));
            let internal = new_user_data.internal.clone();

            ffi_dispatch!(
                WAYLAND_SERVER_HANDLE,
                wl_resource_set_dispatcher,
                self.ptr,
                self::native_machinery::resource_dispatcher::<I>,
                &::wayland_sys::RUST_MANAGED as *const _ as *const _,
                Box::into_raw(new_user_data) as *mut _,
                Some(self::native_machinery::resource_destroy::<I>)
            );

            Resource {
                _i: ::std::marker::PhantomData,
                internal: Some(internal),
                ptr: self.ptr,
            }
        }
    }

    #[cfg(feature = "native_lib")]
    /// Get a raw pointer to the underlying wayland object
    ///
    /// Retrieve a pointer to the object from the `libwayland-server.so` library.
    /// You will mostly need it to interface with C libraries needing access
    /// to wayland objects.
    ///
    /// Use this if you need to pass an unimplemented object to the C library
    /// you are interfacing with.
    pub fn c_ptr(&self) -> *mut wl_resource {
        self.ptr
    }

    #[cfg(feature = "native_lib")]
    /// Create a `NewResource` instance from a C pointer.
    ///
    /// By doing so, you assert that this wayland object was newly created and
    /// can be safely implemented. As implementing it will overwrite any previously
    /// associated data or implementation, this can cause weird errors akin to
    /// memory corruption if it was not the case.
    pub unsafe fn from_c_ptr(ptr: *mut wl_resource) -> Self {
        NewResource {
            _i: ::std::marker::PhantomData,
            ptr: ptr,
        }
    }
}

#[cfg(feature = "native_lib")]
mod native_machinery {
    use wayland_sys::common::*;
    use wayland_sys::server::*;

    use std::sync::Arc;
    use std::sync::atomic::Ordering;
    use std::os::raw::{c_int, c_void};

    use super::Resource;

    use wayland_commons::{Implementation, Interface, MessageGroup};

    pub(crate) struct ResourceUserData<I: Interface> {
        _i: ::std::marker::PhantomData<*const I>,
        pub(crate) internal: Arc<super::ResourceInternal>,
        implem: Option<
            (
                Box<Implementation<Resource<I>, I::Request>>,
                Option<Box<FnMut(Resource<I>, Box<Implementation<Resource<I>, I::Request>>)>>,
            ),
        >,
    }

    impl<I: Interface> ResourceUserData<I> {
        pub(crate) fn new<Impl, Dest>(implem: Impl, destructor: Option<Dest>) -> ResourceUserData<I>
        where
            Impl: Implementation<Resource<I>, I::Request> + 'static,
            Dest: FnMut(Resource<I>, Box<Implementation<Resource<I>, I::Request>>) + 'static,
        {
            ResourceUserData {
                _i: ::std::marker::PhantomData,
                internal: Arc::new(super::ResourceInternal::new()),
                implem: Some((Box::new(implem), destructor.map(|d| Box::new(d) as Box<_>))),
            }
        }

        pub(crate) fn is_impl<Impl>(&self) -> bool
        where
            Impl: Implementation<Resource<I>, I::Request> + 'static,
        {
            self.implem
                .as_ref()
                .map(|implem| implem.0.is::<Impl>())
                .unwrap_or(false)
        }
    }

    pub(crate) unsafe extern "C" fn resource_dispatcher<I: Interface>(
        _implem: *const c_void,
        resource: *mut c_void,
        opcode: u32,
        _msg: *const wl_message,
        args: *const wl_argument,
    ) -> c_int
    where
        I: Interface,
    {
        let resource = resource as *mut wl_resource;

        // We don't need to worry about panic-safeness, because if there is a panic,
        // we'll abort the process, so no access to corrupted data is possible.
        let ret = ::std::panic::catch_unwind(move || {
            // parse the message:
            let msg = I::Request::from_raw_c(resource as *mut _, opcode, args)?;
            let must_destroy = msg.is_destructor();
            // create the resource object
            let resource_obj = super::Resource::<I>::from_c_ptr(resource);
            // retrieve the impl
            let user_data = ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_resource_get_user_data, resource);
            {
                let user_data = &mut *(user_data as *mut ResourceUserData<I>);
                let implem = user_data.implem.as_mut().unwrap();
                let &mut (ref mut implem_func, _) = implem;
                if must_destroy {
                    user_data.internal.alive.store(false, Ordering::Release);
                }
                // call the impl
                implem_func.receive(msg, resource_obj);
            }
            if must_destroy {
                // final cleanup
                ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_resource_destroy, resource);
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

    pub(crate) unsafe extern "C" fn resource_destroy<I: Interface>(resource: *mut wl_resource) {
        let user_data = ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_resource_get_user_data, resource);

        // We don't need to worry about panic-safeness, because if there is a panic,
        // we'll abort the process, so no access to corrupted data is possible.
        let ret = ::std::panic::catch_unwind(move || {
            let mut user_data = Box::from_raw(user_data as *mut ResourceUserData<I>);
            user_data.internal.alive.store(false, Ordering::Release);
            let implem = user_data.implem.as_mut().unwrap();
            let &mut (ref mut implem_func, ref mut destructor) = implem;
            if let Some(mut dest_func) = destructor.take() {
                let retrieved_implem = ::std::mem::replace(implem_func, Box::new(|_, _| {}));
                let resource_obj = super::Resource::<I>::from_c_ptr(resource);
                dest_func(resource_obj, retrieved_implem);
            }
        });

        if let Err(_) = ret {
            eprintln!(
                "[wayland-client error] A destructor for {} panicked.",
                I::NAME
            );
            ::libc::abort()
        }
    }
}
