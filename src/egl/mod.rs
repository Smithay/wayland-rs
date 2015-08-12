//! The EGL wayland protocol.
//!
//! This modules handles the creation of EGL surfaces in wayland.
//!
//! This module depends on he presence of `libwayland-egl.so` in the system,
//! which should be provided by the graphics driver. If the library is not
//! present, all methods of `EGLSurface` will panic.
//!
//! If the EGL support is not mandatory for your use, you can test the presence
//! of the library with the function `is_egl_available()` before calling the
//! other methods.

use core::Surface;
use core::compositor::WSurface;

use ffi::FFI;

#[cfg(feature = "dlopen")]
use self::eglffi::{WAYLAND_EGL_OPTION, WAYLAND_EGL_HANDLE};
#[cfg(not(feature = "dlopen"))]
use self::eglffi::{wl_egl_window_create, wl_egl_window_destroy,
                   wl_egl_window_resize, wl_egl_window_get_attached_size};

pub use self::eglffi::wl_egl_window;

#[cfg(feature = "dlopen")]
/// Returns whether the library `libwayland-egl.so` has been found and could be loaded.
///
/// This function is only presend with the feature `dlopen` activated.
pub fn is_egl_available() -> bool {
    WAYLAND_EGL_OPTION.is_some()
}

pub struct EGLSurface {
    ptr: *mut wl_egl_window,
    surface: WSurface
}

/// EGLSurface is self owned
unsafe impl Send for EGLSurface {}
/// The wayland library guaranties this.
unsafe impl Sync for EGLSurface {}

impl EGLSurface {
    /// Creates a new EGL surface on a wayland surface.
    pub fn new(surface: WSurface, width: i32, height: i32) -> EGLSurface {
        let ptr = unsafe {
            ffi_dispatch!(WAYLAND_EGL_HANDLE, wl_egl_window_create,
                            surface.ptr_mut(), width, height)
        };
        EGLSurface {
            ptr: ptr,
            surface: surface
        }
    }

    /// Destroys the EGL association to this `WSurface` and returns it.
    pub fn destroy(mut self) -> WSurface {
        use std::mem::{forget, replace, uninitialized};
        unsafe {
            let surface = replace(&mut self.surface, uninitialized());
            ffi_dispatch!(WAYLAND_EGL_HANDLE,wl_egl_window_destroy,self.ptr);
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

    /// Resizes the egl surface.
    ///
    /// `(dx, dy)` are the new coordinates of the top-left corner, relative to the current
    /// position.
    /// It allow you to control the direction of the growth or the shrinking of the surface.
    pub fn resize(&self, width: i32, height: i32, dx: i32, dy: i32) {
        unsafe {
            ffi_dispatch!(WAYLAND_EGL_HANDLE, wl_egl_window_resize,
                self.ptr, width, height, dx, dy)
        }
    }

    /// The size of the EGL buffer attached to this surface.
    pub fn get_attached_size(&self) -> (i32, i32) {
        let mut width = 0;
        let mut height = 0;
        unsafe {
            ffi_dispatch!(WAYLAND_EGL_HANDLE, wl_egl_window_get_attached_size,
                self.ptr,
                &mut width as &mut i32,
                &mut height as &mut i32
            );
        }
        (width, height)
    }

}

impl Surface for EGLSurface {
    fn get_wsurface(&self) -> &WSurface {
        &self.surface
    }
}

impl Drop for EGLSurface {
    fn drop(&mut self) {
        unsafe { ffi_dispatch!(WAYLAND_EGL_HANDLE, wl_egl_window_destroy, self.ptr) }
    }
}

impl FFI for EGLSurface {
    type Ptr = wl_egl_window;

    fn ptr(&self) -> *const wl_egl_window {
        self.ptr as *const wl_egl_window
    }

    unsafe fn ptr_mut(&self) -> *mut wl_egl_window {
        self.ptr
    }
}

mod eglffi {
    use ffi::interfaces::surface::wl_surface;

    /// An opaque struct representing a native window for EGL.
    ///
    /// Its sole purpose is to provide a pointer to feed to EGL.
    #[repr(C)] pub struct wl_egl_window;

    external_library!(WaylandEGL, "wayland-egl",
        functions:
            fn wl_egl_window_create(*mut wl_surface, i32, i32) -> *mut wl_egl_window,
            fn wl_egl_window_destroy(*mut wl_egl_window) -> (),
            fn wl_egl_window_resize(*mut wl_egl_window, i32, i32, i32, i32) -> (),
            fn wl_egl_window_get_attached_size(*mut wl_egl_window, *mut i32, *mut i32) -> ()
    );

    #[cfg(feature = "dlopen")]
    lazy_static!(
        pub static ref WAYLAND_EGL_OPTION: Option<WaylandEGL> = { 
            WaylandEGL::open("libwayland-egl.so").ok()
        };
        pub static ref WAYLAND_EGL_HANDLE: &'static WaylandEGL = {
            WAYLAND_EGL_OPTION.as_ref().expect("Library libwayland-egl.so could not be loaded.")
        };
    );
}

#[cfg(test)]
mod tests {
    #![allow(dead_code)]

    use super::EGLSurface;

    fn require_send_sync<T: Send + Sync>() {}

    fn send_sync() {
        require_send_sync::<EGLSurface>();
    }
}