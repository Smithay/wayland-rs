use wayland_sys::server::*;

pub struct Client {
    ptr: *mut wl_client
}

impl Client {
    pub fn ptr(&self) -> *mut wl_client {
        self.ptr
    }

    pub fn post_no_memory(&self) {
        unsafe {
            ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_client_post_no_memory, self.ptr)
        }
    }

    pub unsafe fn from_ptr(ptr: *mut wl_client) -> Client {
        Client { ptr: ptr }
    }
}
