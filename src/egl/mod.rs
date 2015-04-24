//! The EGL wayland protocol.
//!
//! This modules handles the creation of EGL surfaces in wayland.

use core::{WSurface, Surface};

use ffi::interfaces::surface::wl_surface;
use ffi::FFI;

/// An opaque struct representing a native window for EGL.
///
/// Its sole purpose is to provide a pointer to feed to EGL.
#[repr(C)] pub struct wl_egl_window;

#[link(name = "wayland-egl")]
extern {
    fn wl_egl_window_create(surface: *mut wl_surface,
                            width: i32,
                            height: i32
                           ) -> *mut wl_egl_window;
    fn wl_egl_window_destroy(window: *mut wl_egl_window);
    fn wl_egl_window_resize(window: *mut wl_egl_window,
                            width: i32,
                            height: i32,
                            dx: i32,
                            dy: i32);
    fn wl_egl_window_get_attached_size(window: *mut wl_egl_window,
                                       width: *mut i32,
                                       height: *mut i32);
}

pub struct EGLSurface<'a> {
    ptr: *mut wl_egl_window,
    surface: WSurface<'a>
}

impl<'a> EGLSurface<'a> {
    /// Creates a new EGL surface on a wayland surface.
    pub fn new(surface: WSurface<'a>, width: i32, height: i32) -> EGLSurface<'a> {
        let ptr = unsafe { wl_egl_window_create(surface.ptr_mut(), width, height) };
        EGLSurface {
            ptr: ptr,
            surface: surface
        }
    }

    /// Destroys the EGL association to this `WSurface` and returns it.
    pub fn destroy(mut self) -> WSurface<'a> {
        use std::mem::{forget, replace, uninitialized};
        unsafe {
            let surface = replace(&mut self.surface, uninitialized());
            wl_egl_window_destroy(self.ptr);
            forget(self);
            surface
        }
    }

    /// Provides a ptr to the native window to be used for EGL initialization.
    /// 
    /// Keep in mind that the surface will be destroyed when the `EGLSurface`
    /// goes out of scope.
    pub unsafe fn as_native_ptr(&self) -> *mut wl_egl_window {
        self.ptr
    }

    pub fn resize(&self, width: i32, height: i32, dx: i32, dy: i32) {
        unsafe { wl_egl_window_resize(self.ptr, width, height, dx, dy) }
    }

    pub fn get_attached_size(&self) -> (i32, i32) {
        let mut width = 0;
        let mut height = 0;
        unsafe {
            wl_egl_window_get_attached_size(
                self.ptr,
                &mut width as &mut i32,
                &mut height as &mut i32
            );
        }
        (width, height)
    }

}

impl<'a> Surface<'a> for EGLSurface<'a> {
    fn get_wsurface(&self) -> &WSurface<'a> {
        &self.surface
    }
}

impl<'a> Drop for EGLSurface<'a> {
    fn drop(&mut self) {
        unsafe { wl_egl_window_destroy(self.ptr) }
    }
}

impl<'a> FFI for EGLSurface<'a> {
    type Ptr = wl_egl_window;

    fn ptr(&self) -> *const wl_egl_window {
        self.ptr as *const wl_egl_window
    }

    unsafe fn ptr_mut(&self) -> *mut wl_egl_window {
        self.ptr
    }
}
