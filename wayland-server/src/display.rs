use wayland_sys::server::*;

use event_loop::{create_event_loop, EventLoop};

pub struct Display {
    ptr: *mut wl_display,
}

pub fn create_display() -> (Display, EventLoop) {
    unsafe {
        let ptr = ffi_dispatch!(
            WAYLAND_SERVER_HANDLE,
            wl_display_create,
        );
        let el_ptr = ffi_dispatch!(
            WAYLAND_SERVER_HANDLE,
            wl_display_get_event_loop,
            ptr
        );
        (Display { ptr: ptr }, create_event_loop(el_ptr, Some(ptr)))
    }
}

impl Display {
    /// Flush events to the clients
    ///
    /// Will send as many pending events as possible to the respective sockets of the clients.
    /// Will not block, but might not send everything if the socket buffer fills up.
    fn flush_clients(&self) {
        let ret = unsafe { ffi_dispatch!(
            WAYLAND_SERVER_HANDLE,
            wl_display_flush_clients,
            display
        )};
    }
}

impl Drop for Display {
    fn drop(&mut self) {
        unsafe {
            ffi_dispatch!(
                WAYLAND_SERVER_HANDLE,
                wl_display_destroy,
                self.ptr
            );
        }
    }
}
