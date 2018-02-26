use wayland_commons::{Implementation, Interface, MessageGroup};

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
                as *mut self::native_machinery::ResourceUserData;
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
    pub fn implement<ID: 'static>(
        self,
        idata: ID,
        implementation: Implementation<Resource<I>, I::Events, ID>,
        destructor: Option<Implementation<Resource<I>, (), ID>>,
    ) -> Resource<I> {
        #[cfg(not(feature = "native_lib"))]
        {
            unimplemented!()
        }
        #[cfg(feature = "native_lib")]
        {
            let new_user_data = Box::new(self::native_machinery::ResourceUserData::new(
                idata,
                implementation,
                destructor,
            ));
            let internal = new_user_data.internal.clone();

            unsafe {
                ffi_dispatch!(
                    WAYLAND_SERVER_HANDLE,
                    wl_resource_set_dispatcher,
                    self.ptr,
                    self::native_machinery::resource_dispatcher::<I, ID>,
                    &::wayland_sys::RUST_MANAGED as *const _ as *const _,
                    Box::into_raw(new_user_data) as *mut _,
                    Some(self::native_machinery::resource_destroy::<I, ID>)
                );
            }

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

    use std::any::Any;
    use std::sync::Arc;
    use std::sync::atomic::Ordering;
    use std::os::raw::{c_int, c_void};

    use super::{NewResource, Resource};

    use wayland_commons::{Implementation, Interface, MessageGroup};

    pub(crate) struct ResourceUserData {
        pub(crate) internal: Arc<super::ResourceInternal>,
        implem: Option<Box<Any>>,
    }

    impl ResourceUserData {
        pub(crate) fn new<I: Interface, ID: 'static>(
            idata: ID,
            implem: Implementation<Resource<I>, I::Events, ID>,
            destructor: Option<Implementation<Resource<I>, (), ID>>,
        ) -> ResourceUserData {
            ResourceUserData {
                internal: Arc::new(super::ResourceInternal::new()),
                implem: Some(Box::new((implem, idata, destructor)) as Box<Any>),
            }
        }
    }

    pub(crate) unsafe extern "C" fn resource_dispatcher<I: Interface, ID: 'static>(
        _implem: *const c_void,
        resource: *mut c_void,
        opcode: u32,
        msg: *const wl_message,
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
            // create the proxy object
            let resource_obj = super::Resource::<I>::from_c_ptr(resource);
            // retrieve the impl
            let user_data = ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_resource_get_user_data, resource);
            {
                let user_data = &mut *(user_data as *mut ResourceUserData);
                let implem = user_data
                    .implem
                    .as_mut()
                    .unwrap()
                    .downcast_mut::<(
                        Implementation<Resource<I>, I::Requests, ID>,
                        ID,
                        Option<Implementation<Resource<I>, (), ID>>,
                    )>()
                    .unwrap();
                let &mut (ref implem_func, ref mut idata, _) = implem;
                if must_destroy {
                    user_data.internal.alive.store(false, Ordering::Release);
                }
                // call the impl
                implem_func(resource_obj, msg, idata);
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

    pub(crate) unsafe extern "C" fn resource_destroy<I: Interface, ID: 'static>(resource: *mut wl_resource) {
        let user_data = ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_resource_get_user_data, resource);

        // We don't need to worry about panic-safeness, because if there is a panic,
        // we'll abort the process, so no access to corrupted data is possible.
        let ret = ::std::panic::catch_unwind(move || {
            let mut user_data = Box::from_raw(user_data as *mut ResourceUserData);
            user_data.internal.alive.store(false, Ordering::Release);
            let implem = user_data
                .implem
                .as_mut()
                .unwrap()
                .downcast_mut::<(
                    Implementation<Resource<I>, I::Events, ID>,
                    ID,
                    Option<Implementation<Resource<I>, (), ID>>,
                )>()
                .unwrap();
            let &mut (_, ref mut idata, ref destructor) = implem;
            if let &Some(dest_func) = destructor {
                let resource_obj = super::Resource::<I>::from_c_ptr(resource);
                dest_func(resource_obj, (), idata);
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
