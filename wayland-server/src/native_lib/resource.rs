use std::cell::RefCell;
use std::os::raw::{c_int, c_void};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use wayland_sys::common::*;
use wayland_sys::server::*;

use wayland_commons::user_data::UserData;

use crate::{Interface, Main, MessageGroup, Resource};

use super::ClientInner;

pub(crate) struct ResourceInternal {
    alive: AtomicBool,
    user_data: UserData,
}

impl ResourceInternal {
    fn new(user_data: UserData) -> ResourceInternal {
        ResourceInternal {
            alive: AtomicBool::new(true),
            user_data,
        }
    }
}

pub(crate) struct ResourceInner {
    internal: Option<Arc<ResourceInternal>>,
    ptr: *mut wl_resource,
}

unsafe impl Send for ResourceInner {}
unsafe impl Sync for ResourceInner {}

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

        let _c_safety_guard = super::C_SAFETY.lock();

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
            .unwrap_or(true)
    }

    pub(crate) fn version(&self) -> u32 {
        if !self.is_alive() {
            return 0;
        }
        let _c_safety_guard = super::C_SAFETY.lock();
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
        let _c_safety_guard = super::C_SAFETY.lock();
        unsafe {
            let my_client_ptr = ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_resource_get_client, self.ptr);
            let other_client_ptr = ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_resource_get_client, other.ptr);
            my_client_ptr == other_client_ptr
        }
    }

    pub(crate) fn post_error(&self, error_code: u32, msg: String) {
        // If `str` contains an interior null, the actual transmitted message will
        // be truncated at this point.
        let _c_safety_guard = super::C_SAFETY.lock();
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

    pub(crate) fn client(&self) -> Option<ClientInner> {
        if self.is_alive() {
            let _c_safety_guard = super::C_SAFETY.lock();
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
            let _c_safety_guard = super::C_SAFETY.lock();
            unsafe { ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_resource_get_id, self.ptr) }
        } else {
            0
        }
    }

    pub(crate) fn c_ptr(&self) -> *mut wl_resource {
        self.ptr
    }

    pub(crate) unsafe fn init_from_c_ptr<I: Interface + From<Resource<I>> + AsRef<Resource<I>>>(
        ptr: *mut wl_resource,
    ) -> Self {
        let _c_safety_guard = super::C_SAFETY.lock();
        if ptr.is_null() {
            return ResourceInner {
                internal: Some(Arc::new(ResourceInternal {
                    alive: AtomicBool::new(false),
                    user_data: UserData::new(),
                })),
                ptr,
            };
        }

        let new_user_data = Box::new(ResourceUserData::<I>::new());
        let internal = Some(new_user_data.internal.clone());

        ffi_dispatch!(
            WAYLAND_SERVER_HANDLE,
            wl_resource_set_dispatcher,
            ptr,
            resource_dispatcher::<I>,
            &::wayland_sys::RUST_MANAGED as *const _ as *const _,
            Box::into_raw(new_user_data) as *mut _,
            Some(resource_destroy::<I>)
        );

        ResourceInner { internal, ptr }
    }

    pub(crate) unsafe fn from_c_ptr<I: Interface + From<Resource<I>> + AsRef<Resource<I>>>(
        ptr: *mut wl_resource,
    ) -> Self {
        let _c_safety_guard = super::C_SAFETY.lock();
        if ptr.is_null() {
            return ResourceInner {
                internal: Some(Arc::new(ResourceInternal {
                    alive: AtomicBool::new(false),
                    user_data: UserData::new(),
                })),
                ptr,
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
        ResourceInner { internal, ptr }
    }

    pub unsafe fn make_child_for<J: Interface + From<Resource<J>> + AsRef<Resource<J>>>(
        &self,
        id: u32,
    ) -> Option<ResourceInner> {
        let _c_safety_guard = super::C_SAFETY.lock();
        let version = self.version();
        let client_ptr = match self.client() {
            Some(c) => c.ptr(),
            None => return None,
        };
        let new_ptr = ffi_dispatch!(
            WAYLAND_SERVER_HANDLE,
            wl_resource_create,
            client_ptr,
            J::c_interface(),
            version as i32,
            id
        );
        Some(ResourceInner::init_from_c_ptr::<J>(new_ptr))
    }

    pub(crate) fn user_data(&self) -> &UserData {
        lazy_static::lazy_static! {
            static ref INVALID_USERDATA: UserData = {
                let data = UserData::new();
                data.set_threadsafe(|| ());
                data
            };
        };
        if let Some(ref inner) = self.internal {
            &inner.user_data
        } else {
            &INVALID_USERDATA
        }
    }

    pub(crate) fn clone(&self) -> ResourceInner {
        ResourceInner {
            internal: self.internal.clone(),
            ptr: self.ptr,
        }
    }

    pub fn assign<I, E>(&self, filter: crate::Filter<E>)
    where
        I: Interface + AsRef<Resource<I>> + From<Resource<I>>,
        E: From<(Main<I>, I::Request)> + 'static,
        I::Request: MessageGroup<Map = super::ResourceMap>,
    {
        let _c_safety_guard = super::C_SAFETY.lock();
        unsafe {
            let user_data = ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_resource_get_user_data, self.ptr)
                as *mut ResourceUserData<I>;
            if let Ok(ref mut guard) = (*user_data).implem.try_borrow_mut() {
                **guard = Some(Box::new(move |evt, obj| filter.send((obj, evt).into())));
            } else {
                panic!("Re-assigning an object from within its own callback is not supported.");
            }
        }
    }

    pub fn assign_destructor<I, E>(&self, filter: crate::Filter<E>)
    where
        I: Interface + AsRef<Resource<I>> + From<Resource<I>>,
        E: From<Resource<I>> + 'static,
    {
        let _c_safety_guard = super::C_SAFETY.lock();
        unsafe {
            let user_data = ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_resource_get_user_data, self.ptr)
                as *mut ResourceUserData<I>;
            (*user_data).destructor = Some(Box::new(move |obj| filter.send(obj.into())));
        }
    }
}

type BoxedHandler<I> = Box<dyn Fn(<I as Interface>::Request, Main<I>)>;
type BoxedDest<I> = Box<dyn Fn(Resource<I>)>;

pub(crate) struct ResourceUserData<I: Interface + From<Resource<I>> + AsRef<Resource<I>>> {
    _i: ::std::marker::PhantomData<*const I>,
    pub(crate) internal: Arc<ResourceInternal>,
    implem: RefCell<Option<BoxedHandler<I>>>,
    destructor: Option<BoxedDest<I>>,
}

impl<I: Interface + From<Resource<I>> + AsRef<Resource<I>>> ResourceUserData<I> {
    pub(crate) fn new() -> ResourceUserData<I> {
        ResourceUserData {
            _i: ::std::marker::PhantomData,
            internal: Arc::new(ResourceInternal::new(UserData::new())),
            implem: RefCell::new(None),
            destructor: None,
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
    I: Interface + From<Resource<I>> + AsRef<Resource<I>>,
{
    let resource = resource as *mut wl_resource;

    // We don't need to worry about panic-safeness, because if there is a panic,
    // we'll abort the process, so no access to corrupted data is possible.
    let ret = ::std::panic::catch_unwind(move || {
        // parse the message:
        let msg = I::Request::from_raw_c(resource as *mut _, opcode, args)?;
        let must_destroy = msg.is_destructor();
        // create the resource object
        let resource_obj = Main::<I>::wrap(ResourceInner::from_c_ptr::<I>(resource));
        // retrieve the impl
        let user_data = ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_resource_get_user_data, resource);
        {
            let user_data = &mut *(user_data as *mut ResourceUserData<I>);
            let implem = user_data.implem.borrow();

            if must_destroy {
                user_data.internal.alive.store(false, Ordering::Release);
            }

            match implem.as_ref() {
                Some(ref implem_func) => implem_func(msg, resource_obj),
                None => {
                    eprintln!(
                        "[wayland-server] Request received for an object not associated to any filter: {}@{}",
                        I::NAME,
                        resource_obj.as_ref().id()
                    );
                    resource_obj
                        .as_ref()
                        .post_error(2, "Server-side bug, sorry.".into());
                    return Ok(());
                }
            }
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

pub(crate) unsafe extern "C" fn resource_destroy<I>(resource: *mut wl_resource)
where
    I: Interface + From<Resource<I>> + AsRef<Resource<I>>,
{
    let user_data = ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_resource_get_user_data, resource);

    // We don't need to worry about panic-safeness, because if there is a panic,
    // we'll abort the process, so no access to corrupted data is possible.
    let ret = ::std::panic::catch_unwind(move || {
        let mut user_data = Box::from_raw(user_data as *mut ResourceUserData<I>);
        user_data.internal.alive.store(false, Ordering::Release);
        if let Some(dest_func) = user_data.destructor.take() {
            let resource_obj = Resource::<I>::from_c_ptr(resource);
            dest_func(resource_obj);
        }
    });

    if ret.is_err() {
        eprintln!("[wayland-client error] A destructor for {} panicked.", I::NAME);
        ::libc::abort()
    }
}
