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

pub struct Resource<I: Interface> {
    _i: ::std::marker::PhantomData<*const I>,
    #[cfg(not(feature = "native_lib"))] internal: Arc<ResourceInternal>,
    #[cfg(feature = "native_lib")] internal: Option<Arc<ResourceInternal>>,
    #[cfg(feature = "native_lib")] ptr: *mut wl_resource,
}

impl<I: Interface> Resource<I> {
    pub fn send(&self, msg: I::Events) {
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

    // returns false if external
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

    pub fn clone(&self) -> Resource<I> {
        Resource {
            _i: ::std::marker::PhantomData,
            internal: self.internal.clone(),
            #[cfg(feature = "native_lib")]
            ptr: self.ptr,
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
    pub fn c_ptr(&self) -> *mut wl_resource {
        self.ptr
    }

    #[cfg(feature = "native_lib")]
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
}

pub struct NewResource<I: Interface> {
    _i: ::std::marker::PhantomData<*const I>,
    #[cfg(feature = "native_lib")] ptr: *mut wl_resource,
}

impl<I: Interface + 'static> NewResource<I> {
    /// Implement this resource using given function, destructor, and implementation data.
    pub fn implement<Impl, Dest>(self, implementation: Impl, destructor: Option<Dest>) -> Resource<I>
    where
        Impl: Implementation<Resource<I>, I::Requests> + Send + 'static,
        Dest: FnMut(Resource<I>, Box<Implementation<Resource<I>, I::Requests>>) + Send + 'static,
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
        Impl: Implementation<Resource<I>, I::Requests> + 'static,
        Dest: FnMut(Resource<I>, Box<Implementation<Resource<I>, I::Requests>>) + 'static,
    {
        let _ = token;
        self.implement_inner(implementation, destructor)
    }

    unsafe fn implement_inner<Impl, Dest>(self, implementation: Impl, destructor: Option<Dest>) -> Resource<I>
    where
        Impl: Implementation<Resource<I>, I::Requests> + 'static,
        Dest: FnMut(Resource<I>, Box<Implementation<Resource<I>, I::Requests>>) + 'static,
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
                Box<Implementation<Resource<I>, I::Requests>>,
                Option<Box<FnMut(Resource<I>, Box<Implementation<Resource<I>, I::Requests>>)>>,
            ),
        >,
    }

    impl<I: Interface> ResourceUserData<I> {
        pub(crate) fn new<Impl, Dest>(implem: Impl, destructor: Option<Dest>) -> ResourceUserData<I>
        where
            Impl: Implementation<Resource<I>, I::Requests> + 'static,
            Dest: FnMut(Resource<I>, Box<Implementation<Resource<I>, I::Requests>>) + 'static,
        {
            ResourceUserData {
                _i: ::std::marker::PhantomData,
                internal: Arc::new(super::ResourceInternal::new()),
                implem: Some((Box::new(implem), destructor.map(|d| Box::new(d) as Box<_>))),
            }
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
            let msg = I::Requests::from_raw_c(resource as *mut _, opcode, args)?;
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
