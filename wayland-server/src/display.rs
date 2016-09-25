use wayland_sys::server::*;

use event_loop::{create_event_loop, EventLoop};

/// A wayland socket
///
/// This represents a socket your compositor can receive clients on.
pub struct Display {
    ptr: *mut wl_display,
}

/// Create a new display
///
/// Create a socket to listen on for clients. By default use the name
/// `wayland-0`, if not overwriten by the environment variable `WAYLAND_DISPLAY`.
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
    pub fn flush_clients(&self) {
        unsafe { ffi_dispatch!(
            WAYLAND_SERVER_HANDLE,
            wl_display_flush_clients,
            self.ptr
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
