use std::cell::RefCell;
use std::ffi::CStr;
use std::os::raw::c_void;
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use wayland_commons::ThreadGuard;
use wayland_sys::server::*;

use super::resource::ResourceInner;
use crate::{client::Credentials, DispatchData, Interface, Resource, UserDataMap};

type BoxedDest = Box<dyn FnMut(Arc<UserDataMap>, DispatchData<'_>) + 'static>;

pub(crate) struct ClientInternal {
    alive: AtomicBool,
    user_data_map: Arc<UserDataMap>,
    destructors: ThreadGuard<RefCell<Vec<BoxedDest>>>,
    safe_thread: std::thread::ThreadId,
}

impl ClientInternal {
    fn new() -> ClientInternal {
        ClientInternal {
            alive: AtomicBool::new(true),
            user_data_map: Arc::new(UserDataMap::new()),
            destructors: ThreadGuard::new(RefCell::new(Vec::new())),
            safe_thread: std::thread::current().id(),
        }
    }
}

#[derive(Clone)]
pub(crate) struct ClientInner {
    ptr: *mut wl_client,
    internal: Arc<ClientInternal>,
}

unsafe impl Send for ClientInner {}
unsafe impl Sync for ClientInner {}

impl ClientInner {
    pub(crate) unsafe fn from_ptr(ptr: *mut wl_client) -> ClientInner {
        let _c_safety_guard = super::C_SAFETY.lock();
        // check if we are already registered
        let listener = ffi_dispatch!(
            WAYLAND_SERVER_HANDLE,
            wl_client_get_destroy_listener,
            ptr,
            client_destroy
        );
        if listener.is_null() {
            // need to init this client
            let listener = signal::rust_listener_create(client_destroy);
            let internal = Arc::new(ClientInternal::new());
            signal::rust_listener_set_user_data(
                listener,
                Box::into_raw(Box::new(internal.clone())) as *mut c_void,
            );
            ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_client_add_destroy_listener, ptr, listener);
            ClientInner { ptr, internal }
        } else {
            // client already initialized
            let internal =
                signal::rust_listener_get_user_data(listener) as *mut Arc<ClientInternal>;
            ClientInner { ptr, internal: (*internal).clone() }
        }
    }

    pub(crate) fn ptr(&self) -> *mut wl_client {
        self.ptr
    }

    pub(crate) fn alive(&self) -> bool {
        self.internal.alive.load(Ordering::Acquire)
    }

    pub(crate) fn equals(&self, other: &ClientInner) -> bool {
        Arc::ptr_eq(&self.internal, &other.internal)
    }

    pub(crate) fn flush(&self) {
        if !self.alive() {
            return;
        }
        let _c_safety_guard = super::C_SAFETY.lock();
        unsafe {
            ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_client_flush, self.ptr);
        }
    }

    pub(crate) fn kill(&self) {
        if !self.alive() {
            return;
        }
        let _c_safety_guard = super::C_SAFETY.lock();
        unsafe {
            ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_client_destroy, self.ptr);
        }
    }

    pub(crate) fn credentials(&self) -> Option<Credentials> {
        if !self.alive() {
            return None;
        }
        let _c_safety_guard = super::C_SAFETY.lock();
        unsafe {
            let mut credentials = Credentials { pid: 0, uid: 0, gid: 0 };

            ffi_dispatch!(
                WAYLAND_SERVER_HANDLE,
                wl_client_get_credentials,
                self.ptr,
                &mut credentials.pid,
                &mut credentials.uid,
                &mut credentials.gid
            );
            Some(credentials)
        }
    }

    pub(crate) fn user_data_map(&self) -> &UserDataMap {
        &self.internal.user_data_map
    }

    pub(crate) fn add_destructor<F: FnOnce(Arc<UserDataMap>, DispatchData<'_>) + 'static>(
        &self,
        destructor: F,
    ) {
        if self.internal.safe_thread != std::thread::current().id() {
            panic!("Can only add a destructor from the thread hosting the Display.");
        }
        let _c_safety_guard = super::C_SAFETY.lock();
        // Wrap the FnOnce in an FnMut because Box<FnOnce()> does not work
        // currently =(
        let mut opt_dest = Some(destructor);
        self.internal.destructors.get().borrow_mut().push(Box::new(move |data_map, data| {
            if let Some(dest) = opt_dest.take() {
                dest(data_map, data);
            }
        }))
    }

    pub(crate) fn create_resource<I: Interface + From<Resource<I>> + AsRef<Resource<I>>>(
        &self,
        version: u32,
    ) -> Option<ResourceInner> {
        if self.internal.safe_thread != std::thread::current().id() {
            panic!("Can only create ressources from the thread hosting the Display.");
        }
        if !self.alive() {
            return None;
        }
        let _c_safety_guard = super::C_SAFETY.lock();
        unsafe {
            let ptr = ffi_dispatch!(
                WAYLAND_SERVER_HANDLE,
                wl_resource_create,
                self.ptr(),
                I::c_interface(),
                version as i32,
                0
            );
            Some(ResourceInner::init_from_c_ptr::<I>(ptr))
        }
    }

    pub(crate) fn get_resource<I: Interface + From<Resource<I>> + AsRef<Resource<I>>>(
        &self,
        id: u32,
    ) -> Option<ResourceInner> {
        let _c_safety_guard = super::C_SAFETY.lock();
        unsafe {
            let ptr = ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_client_get_object, self.ptr(), id);
            if ptr.is_null() {
                return None;
            }
            let interface = ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_resource_get_class, ptr);
            let requested_interface = (*I::c_interface()).name;
            if CStr::from_ptr(interface) != CStr::from_ptr(requested_interface) {
                // interface of this resource is not the requested one
                return None;
            }
            Some(ResourceInner::from_c_ptr::<I>(ptr))
        }
    }
}
impl PartialEq for ClientInner {
    fn eq(&self, other: &Self) -> bool {
        self.equals(other)
    }
}

unsafe extern "C" fn client_destroy(listener: *mut wl_listener, _data: *mut c_void) {
    let internal =
        Box::from_raw(signal::rust_listener_get_user_data(listener) as *mut Arc<ClientInternal>);
    signal::rust_listener_set_user_data(listener, ptr::null_mut());
    // Store that we are dead
    internal.alive.store(false, Ordering::Release);

    let mut destructors = internal.destructors.get().borrow_mut();
    if super::DISPATCH_DATA.is_set() {
        super::DISPATCH_DATA.with(|disp_data| {
            let mut disp_data = disp_data.borrow_mut();
            for mut destructor in destructors.drain(..) {
                destructor(internal.user_data_map.clone(), disp_data.reborrow());
            }
        });
    } else {
        for mut destructor in destructors.drain(..) {
            destructor(internal.user_data_map.clone(), DispatchData::wrap(&mut ()));
        }
    }

    signal::rust_listener_destroy(listener);
}
