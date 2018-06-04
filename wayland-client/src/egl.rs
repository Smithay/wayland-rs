//! EGL utilities
//!
//! This module contains bindings to the `libwayland-egl.so` library.
//!
//! This library is used to interface with the OpenGL stack, and creating
//! EGL surfaces from a wayland surface.
//!
//! See WlEglSurface documentation for details.

use protocol::wl_surface::WlSurface;
use std::os::raw::c_void;
use wayland_sys::client::wl_proxy;
use wayland_sys::egl::*;
use Proxy;

/// Checks if the wayland-egl lib is available and can be used
///
/// Trying to create an `WlEglSurface` while this function returns
/// `false` will result in a panic.
pub fn is_available() -> bool {
    is_lib_available()
}

unsafe impl Send for WlEglSurface {}
unsafe impl Sync for WlEglSurface {}

/// EGL surface
///
/// This object is a simple wrapper around a `WlSurface` to add the EGL
/// capabilities. Just use the `ptr` method once this object is created
/// to get the window pointer your OpenGL library is needing to initialize the
/// EGL context (you'll most likely need the display ptr as well, that you can
/// get via the `ptr` method of the `Proxy` trait on the `WlDisplay` object).
pub struct WlEglSurface {
    ptr: *mut wl_egl_window,
}

impl WlEglSurface {
    /// Create an EGL surface from a wayland surface
    pub fn new(surface: &Proxy<WlSurface>, width: i32, height: i32) -> WlEglSurface {
        unsafe { WlEglSurface::new_from_raw(surface.c_ptr(), width, height) }
    }

    /// Create an EGL surface from a raw pointer to a wayland surface
    ///
    /// This function is unsafe because `surface` must be a valid wl_surface pointer
    pub unsafe fn new_from_raw(surface: *mut wl_proxy, width: i32, height: i32) -> WlEglSurface {
        let ptr = ffi_dispatch!(WAYLAND_EGL_HANDLE, wl_egl_window_create, surface, width, height);
        WlEglSurface { ptr: ptr }
    }

    /// Fetch current size of the EGL surface
    pub fn get_size(&self) -> (i32, i32) {
        let mut w = 0i32;
        let mut h = 0i32;
        unsafe {
            ffi_dispatch!(
                WAYLAND_EGL_HANDLE,
                wl_egl_window_get_attached_size,
                self.ptr,
                &mut w as *mut i32,
                &mut h as *mut i32
            );
        }
        (w, h)
    }

    /// Resize the EGL surface
    ///
    /// The two first arguments `(width, height)` are the new size of
    /// the surface, the two others `(dx, dy)` represent the displacement
    /// of the top-left corner of the surface. It allows you to control the
    /// direction of the resizing if necessary.
    pub fn resize(&self, width: i32, height: i32, dx: i32, dy: i32) {
        unsafe {
            ffi_dispatch!(
                WAYLAND_EGL_HANDLE,
                wl_egl_window_resize,
                self.ptr,
                width,
                height,
                dx,
                dy
            )
        }
    }

    /// Raw pointer to the EGL surface
    ///
    /// You'll need this pointer to initialize the EGL context in your
    /// favourite OpenGL lib.
    pub fn ptr(&self) -> *const c_void {
        self.ptr as *const c_void
    }
}

impl Drop for WlEglSurface {
    fn drop(&mut self) {
        unsafe {
            ffi_dispatch!(WAYLAND_EGL_HANDLE, wl_egl_window_destroy, self.ptr);
        }
    }
}
