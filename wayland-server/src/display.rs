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
