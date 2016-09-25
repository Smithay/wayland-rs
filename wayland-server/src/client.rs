use wayland_sys::server::*;

/// A wayland client connected to your server
pub struct Client {
    ptr: *mut wl_client
}

impl Client {
    /// Get a pointer to the C wl_client object
    ///
    /// You may need it for FFI with C libraries.
    pub fn ptr(&self) -> *mut wl_client {
        self.ptr
    }

    /// Post a "no memory" message to the client
    ///
    /// This will effectively kill this client's connection, and invalidates all its
    /// objects.
    pub fn post_no_memory(&self) {
        unsafe {
            ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_client_post_no_memory, self.ptr)
        }
    }

    /// Create a client object from a pointer
    pub unsafe fn from_ptr(ptr: *mut wl_client) -> Client {
        Client { ptr: ptr }
    }
}
