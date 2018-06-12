use std::os::raw::c_void;
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};
use std::sync::Arc;

use wayland_sys::server::*;

pub(crate) struct ClientInternal {
    alive: AtomicBool,
    user_data: AtomicPtr<()>,
    destructor: AtomicPtr<()>,
}

impl ClientInternal {
    fn new() -> ClientInternal {
        ClientInternal {
            alive: AtomicBool::new(true),
            user_data: AtomicPtr::new(::std::ptr::null_mut()),
            destructor: AtomicPtr::new(::std::ptr::null_mut()),
        }
    }
}

#[derive(Clone)]
pub(crate) struct ClientInner {
    ptr: *mut wl_client,
    internal: Arc<ClientInternal>,
}

impl ClientInner {
    pub(crate) unsafe fn from_ptr(ptr: *mut wl_client) -> ClientInner {
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
            ffi_dispatch!(
                WAYLAND_SERVER_HANDLE,
                wl_client_add_destroy_listener,
                ptr,
                listener
            );
            ClientInner { ptr, internal }
        } else {
            // client already initialized
            let internal = signal::rust_listener_get_user_data(listener) as *mut Arc<ClientInternal>;
            ClientInner {
                ptr,
                internal: (*internal).clone(),
            }
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
        unsafe {
            ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_client_flush, self.ptr);
        }
    }

    pub(crate) fn kill(&self) {
        if !self.alive() {
            return;
        }
        unsafe {
            ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_client_destroy, self.ptr);
        }
    }

    pub(crate) fn set_user_data(&self, data: *mut ()) {
        self.internal.user_data.store(data, Ordering::Release);
    }

    pub(crate) fn get_user_data(&self) -> *mut () {
        self.internal.user_data.load(Ordering::Acquire)
    }

    pub(crate) fn set_destructor(&self, destructor: fn(*mut ())) {
        self.internal
            .destructor
            .store(destructor as *const () as *mut (), Ordering::Release);
    }
}

unsafe extern "C" fn client_destroy(listener: *mut wl_listener, _data: *mut c_void) {
    let internal = Box::from_raw(signal::rust_listener_get_user_data(listener) as *mut Arc<ClientInternal>);
    signal::rust_listener_set_user_data(listener, ptr::null_mut());
    // Store that we are dead
    internal.alive.store(false, Ordering::Release);
    // TODO: call the user callback
    let destructor = internal.destructor.load(Ordering::Acquire);
    if !destructor.is_null() {
        let destructor_func: fn(*mut ()) = ::std::mem::transmute(destructor);
        destructor_func(internal.user_data.load(Ordering::Acquire));
    }
    signal::rust_listener_destroy(listener);
}
