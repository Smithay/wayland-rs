use std::os::raw::{c_int, c_void};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use wayland_sys::common::*;
use wayland_sys::server::*;

use wayland_commons::utils::UserData;

use {Interface, MessageGroup, Resource};

use super::ClientInner;

pub(crate) struct ResourceInternal {
    alive: AtomicBool,
    user_data: Arc<UserData>,
}

impl ResourceInternal {
    fn new(user_data: UserData) -> ResourceInternal {
        ResourceInternal {
            alive: AtomicBool::new(true),
            user_data: Arc::new(user_data),
        }
    }
}

pub(crate) struct ResourceInner {
    internal: Option<Arc<ResourceInternal>>,
    ptr: *mut wl_resource,
    // this field is only a workaround for https://github.com/rust-lang/rust/issues/50153
    // this bug is fixed since 1.28.0, the hack can be removed once this becomes
    // the least supported version
    _hack: (bool, bool),
}

impl ResourceInner {
    pub(crate) fn send<I: Interface>(&self, msg: I::Event) {
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
                WAYLAND_SERVER_HANDLE,
                wl_resource_post_event_array,
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
                ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_resource_destroy, self.ptr);
            }
        }
    }

    pub(crate) fn is_alive(&self) -> bool {
        self.internal
            .as_ref()
            .map(|i| i.alive.load(Ordering::Acquire))
            .unwrap_or(false)
    }

    pub(crate) fn version(&self) -> u32 {
        if !self.is_alive() {
            return 0;
        }
        unsafe { ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_resource_get_version, self.ptr) as u32 }
    }

    pub(crate) fn is_external(&self) -> bool {
        self.internal.is_none()
    }

    pub(crate) fn equals(&self, other: &ResourceInner) -> bool {
        match (&self.internal, &other.internal) {
            (&Some(ref my_inner), &Some(ref other_inner)) => Arc::ptr_eq(my_inner, other_inner),
            (&None, &None) => self.ptr == other.ptr,
            _ => false,
        }
    }

    pub(crate) fn same_client_as(&self, other: &ResourceInner) -> bool {
        if !(self.is_alive() && other.is_alive()) {
            return false;
        }
        unsafe {
            let my_client_ptr = ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_resource_get_client, self.ptr);
            let other_client_ptr = ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_resource_get_client, other.ptr);
            my_client_ptr == other_client_ptr
        }
    }

    pub(crate) fn post_error(&self, error_code: u32, msg: String) {
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

    pub(crate) fn get_user_data<UD>(&self) -> Option<&UD>
    where
        UD: 'static,
    {
        if let Some(ref inner) = self.internal {
            inner.user_data.get()
        } else {
            None
        }
    }

    pub(crate) fn client(&self) -> Option<ClientInner> {
        if self.is_alive() {
            unsafe {
                let client_ptr = ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_resource_get_client, self.ptr);
                Some(ClientInner::from_ptr(client_ptr))
            }
        } else {
            None
        }
    }

    pub(crate) fn id(&self) -> u32 {
        if self.is_alive() {
            unsafe { ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_resource_get_id, self.ptr) }
        } else {
            0
        }
    }

    pub(crate) fn c_ptr(&self) -> *mut wl_resource {
        self.ptr
    }

    pub(crate) unsafe fn from_c_ptr<I: Interface + From<Resource<I>>>(ptr: *mut wl_resource) -> Self {
        if ptr.is_null() {
            return ResourceInner {
                internal: Some(Arc::new(ResourceInternal {
                    alive: AtomicBool::new(false),
                    user_data: Arc::new(UserData::empty()),
                })),
                ptr,
                _hack: (false, false),
            };
        }

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
                as *mut ResourceUserData<I>;
            Some((*user_data).internal.clone())
        } else {
            None
        };
        ResourceInner {
            internal,
            ptr,
            _hack: (false, false),
        }
    }

    pub(crate) fn clone(&self) -> ResourceInner {
        ResourceInner {
            internal: self.internal.clone(),
            ptr: self.ptr,
            _hack: (false, false),
        }
    }
}

pub(crate) struct NewResourceInner {
    ptr: *mut wl_resource,
}

impl NewResourceInner {
    pub(crate) unsafe fn implement<I: Interface + From<Resource<I>>, F, Dest>(
        self,
        implementation: F,
        destructor: Option<Dest>,
        user_data: UserData,
    ) -> ResourceInner
    where
        F: FnMut(I::Request, I) + 'static,
        Dest: FnMut(I) + 'static,
    {
        let new_user_data = Box::new(ResourceUserData::new(implementation, destructor, user_data));
        let internal = new_user_data.internal.clone();

        ffi_dispatch!(
            WAYLAND_SERVER_HANDLE,
            wl_resource_set_dispatcher,
            self.ptr,
            resource_dispatcher::<I>,
            &::wayland_sys::RUST_MANAGED as *const _ as *const _,
            Box::into_raw(new_user_data) as *mut _,
            Some(resource_destroy::<I>)
        );

        ResourceInner {
            internal: Some(internal),
            ptr: self.ptr,
            _hack: (false, false),
        }
    }

    pub(crate) fn c_ptr(&self) -> *mut wl_resource {
        self.ptr
    }

    pub(crate) unsafe fn from_c_ptr(ptr: *mut wl_resource) -> Self {
        NewResourceInner { ptr }
    }
}

type BoxedHandler<I> = Box<FnMut(<I as Interface>::Request, I)>;
type BoxedDest<I> = Box<FnMut(I)>;

pub(crate) struct ResourceUserData<I: Interface + From<Resource<I>>> {
    _i: ::std::marker::PhantomData<*const I>,
    pub(crate) internal: Arc<ResourceInternal>,
    implem: Option<(BoxedHandler<I>, Option<BoxedDest<I>>)>,
}

impl<I: Interface + From<Resource<I>>> ResourceUserData<I> {
    pub(crate) fn new<F, Dest>(
        implem: F,
        destructor: Option<Dest>,
        user_data: UserData,
    ) -> ResourceUserData<I>
    where
        F: FnMut(I::Request, I) + 'static,
        Dest: FnMut(I) + 'static,
    {
        ResourceUserData {
            _i: ::std::marker::PhantomData,
            internal: Arc::new(ResourceInternal::new(user_data)),
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
    I: Interface + From<Resource<I>>,
{
    let resource = resource as *mut wl_resource;

    // We don't need to worry about panic-safeness, because if there is a panic,
    // we'll abort the process, so no access to corrupted data is possible.
    let ret = ::std::panic::catch_unwind(move || {
        // parse the message:
        let msg = I::Request::from_raw_c(resource as *mut _, opcode, args)?;
        let must_destroy = msg.is_destructor();
        // create the resource object
        let resource_obj = Resource::<I>::from_c_ptr(resource);
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
            implem_func(msg, resource_obj.into());
        }
        if must_destroy {
            // final cleanup
            ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_resource_destroy, resource);
        }
        Ok(())
    });
    // check the return status
    match ret {
        Ok(Ok(())) => 0,
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

pub(crate) unsafe extern "C" fn resource_destroy<I: Interface + From<Resource<I>>>(
    resource: *mut wl_resource,
) {
    let user_data = ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_resource_get_user_data, resource);

    // We don't need to worry about panic-safeness, because if there is a panic,
    // we'll abort the process, so no access to corrupted data is possible.
    let ret = ::std::panic::catch_unwind(move || {
        let mut user_data = Box::from_raw(user_data as *mut ResourceUserData<I>);
        user_data.internal.alive.store(false, Ordering::Release);
        let implem = user_data.implem.as_mut().unwrap();
        let &mut (_, ref mut destructor) = implem;
        if let Some(mut dest_func) = destructor.take() {
            let resource_obj = Resource::<I>::from_c_ptr(resource);
            dest_func(resource_obj.into());
        }
    });

    if ret.is_err() {
        eprintln!("[wayland-client error] A destructor for {} panicked.", I::NAME);
        ::libc::abort()
    }
}
