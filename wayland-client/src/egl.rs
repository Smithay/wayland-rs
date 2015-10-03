use libc::c_void;
use std::ops::Deref;

use wayland_sys::egl::*;
use sys::wayland::client::WlSurface;
use Proxy;

pub struct WlEglSurface {
    ptr: *mut wl_egl_window,
    surface: WlSurface
}

impl WlEglSurface {
    pub fn new(surface: WlSurface, width: i32, height: i32) -> WlEglSurface {
        let ptr = unsafe { ffi_dispatch!(WAYLAND_EGL_HANDLE, wl_egl_window_create,
            surface.ptr(), width, height) };
        WlEglSurface {
            ptr: ptr,
            surface: surface
        }
    }

    pub fn destroy(mut self) -> WlSurface {
        unsafe { ffi_dispatch!(WAYLAND_EGL_HANDLE, wl_egl_window_destroy, self.ptr); }
        let surface = ::std::mem::replace(&mut self.surface, unsafe { ::std::mem::uninitialized() });
        ::std::mem::forget(self);
        surface
    }

    pub fn get_size(&self) -> (i32, i32) {
        let mut w = 0i32;
        let mut h = 0i32;
        unsafe { ffi_dispatch!(WAYLAND_EGL_HANDLE, wl_egl_window_get_attached_size,
            self.ptr, &mut w as *mut i32, &mut h as *mut i32); }
        (w, h)
    }

    pub fn resize(&self, width: i32, height: i32, dx: i32, dy: i32) {
        unsafe { ffi_dispatch!(WAYLAND_EGL_HANDLE, wl_egl_window_resize,
            self.ptr, width, height, dx, dy) }
    }

    pub unsafe fn egl_surfaceptr(&self) -> *mut c_void {
        self.ptr as *mut c_void
    }
}

impl Drop for WlEglSurface {
    fn drop(&mut self) {
        unsafe { ffi_dispatch!(WAYLAND_EGL_HANDLE, wl_egl_window_destroy, self.ptr); }
    }
}

impl Deref for WlEglSurface {
    type Target = WlSurface;
    fn deref(&self) -> &WlSurface {
        &self.surface
    }
}