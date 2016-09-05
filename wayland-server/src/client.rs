use wayland_sys::server::wl_client;

pub struct Client {
    ptr: *mut wl_client
}

impl Client {
    pub fn ptr(&self) -> *mut wl_client {
        self.ptr
    }

    pub unsafe fn from_ptr(ptr: *mut wl_client) -> Client {
        Client { ptr: ptr }
    }
}
