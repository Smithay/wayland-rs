use std::os::raw::{c_int, c_void};
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};
use std::sync::Arc;

use wayland_sys::common::*;
use wayland_sys::server::*;

use {Implementation, Interface, MessageGroup, Resource};

use super::{ClientInner, EventLoopInner};

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

pub(crate) struct ResourceInner {
    internal: Option<Arc<ResourceInternal>>,
    ptr: *mut wl_resource,
    // this field is only a workaround for https://github.com/rust-lang/rust/issues/50153
    _hack: (bool, bool),
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

    pub(crate) fn set_user_data(&self, ptr: *mut ()) {
        if let Some(ref inner) = self.internal {
            inner.user_data.store(ptr, Ordering::Release);
        }
    }

    pub(crate) fn get_user_data(&self) -> *mut () {
        if let Some(ref inner) = self.internal {
            inner.user_data.load(Ordering::Acquire)
        } else {
            ::std::ptr::null_mut()
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
            unsafe {
                ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_resource_get_id, self.ptr)
            }
        } else {
            0
        }
    }

    pub(crate) fn c_ptr(&self) -> *mut wl_resource {
        self.ptr
    }

    pub(crate) unsafe fn from_c_ptr<I: Interface>(ptr: *mut wl_resource) -> Self {
        if ptr.is_null() {
            return ResourceInner {
                internal: Some(Arc::new(ResourceInternal {
                    alive: AtomicBool::new(false),
                    user_data: AtomicPtr::new(::std::ptr::null_mut()),
                })),
                ptr: ptr,
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
            internal: internal,
            ptr: ptr,
            _hack: (false, false),
        }
    }

    pub(crate) fn is_implemented_with<I: Interface, Impl>(&self) -> bool
    where
        Impl: Implementation<Resource<I>, I::Request> + 'static,
    {
        if !self.is_alive() {
            return false;
        }
        let user_data = unsafe {
            let ptr = ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_resource_get_user_data, self.ptr)
                as *mut self::ResourceUserData<I>;
            &*ptr
        };
        user_data.is_impl::<Impl>()
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
    pub(crate) unsafe fn implement<I: Interface, Impl, Dest>(
        self,
        implementation: Impl,
        destructor: Option<Dest>,
        token: Option<&EventLoopInner>,
    ) -> ResourceInner
    where
        Impl: Implementation<Resource<I>, I::Request> + 'static,
        Dest: FnMut(Resource<I>, Box<Implementation<Resource<I>, I::Request>>) + 'static,
    {
        if let Some(token) = token {
            assert!(
                token.matches(self.ptr),
                "Tried to implement a Resource with the wrong LoopToken."
            )
        }
        let new_user_data = Box::new(ResourceUserData::new(implementation, destructor));
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
        NewResourceInner { ptr: ptr }
    }
}

pub(crate) struct ResourceUserData<I: Interface> {
    _i: ::std::marker::PhantomData<*const I>,
    pub(crate) internal: Arc<ResourceInternal>,
    implem: Option<(
        Box<Implementation<Resource<I>, I::Request>>,
        Option<Box<FnMut(Resource<I>, Box<Implementation<Resource<I>, I::Request>>)>>,
    )>,
}

impl<I: Interface> ResourceUserData<I> {
    pub(crate) fn new<Impl, Dest>(implem: Impl, destructor: Option<Dest>) -> ResourceUserData<I>
    where
        Impl: Implementation<Resource<I>, I::Request> + 'static,
        Dest: FnMut(Resource<I>, Box<Implementation<Resource<I>, I::Request>>) + 'static,
    {
        ResourceUserData {
            _i: ::std::marker::PhantomData,
            internal: Arc::new(ResourceInternal::new()),
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
            let resource_obj = Resource::<I>::from_c_ptr(resource);
            dest_func(resource_obj, retrieved_implem);
        }
    });

    if let Err(_) = ret {
        eprintln!("[wayland-client error] A destructor for {} panicked.", I::NAME);
        ::libc::abort()
    }
}
