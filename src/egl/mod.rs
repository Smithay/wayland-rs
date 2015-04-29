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

use core::{WSurface, Surface};

use ffi::FFI;

use self::eglffi::{WAYLAND_EGL_OPTION, WAYLAND_EGL_HANDLE};
pub use self::eglffi::wl_egl_window;


/// Returns whether the library `libwayland-egl.so` has been found and could be loaded.
pub fn is_egl_available() -> bool {
    WAYLAND_EGL_OPTION.is_some()
}

pub struct EGLSurface {
    ptr: *mut wl_egl_window,
    surface: WSurface
}

impl EGLSurface {
    /// Creates a new EGL surface on a wayland surface.
    pub fn new(surface: WSurface, width: i32, height: i32) -> EGLSurface {
        let ptr = unsafe { (WAYLAND_EGL_HANDLE.wl_egl_window_create)(surface.ptr_mut(), width, height) };
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
            (WAYLAND_EGL_HANDLE.wl_egl_window_destroy)(self.ptr);
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
        unsafe { (WAYLAND_EGL_HANDLE.wl_egl_window_resize)(self.ptr, width, height, dx, dy) }
    }

    pub fn get_attached_size(&self) -> (i32, i32) {
        let mut width = 0;
        let mut height = 0;
        unsafe {
            (WAYLAND_EGL_HANDLE.wl_egl_window_get_attached_size)(
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
        unsafe { (WAYLAND_EGL_HANDLE.wl_egl_window_destroy)(self.ptr) }
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

    external_library!(WaylandEGL,
        wl_egl_window_create: unsafe extern fn(surface: *mut wl_surface,
                                               width: i32,
                                               height: i32
                                              ) -> *mut wl_egl_window,
        wl_egl_window_destroy: unsafe extern fn(window: *mut wl_egl_window),
        wl_egl_window_resize: unsafe extern fn(window: *mut wl_egl_window,
                                               width: i32,
                                               height: i32,
                                               dx: i32,
                                               dy: i32),
        wl_egl_window_get_attached_size: unsafe extern fn(window: *mut wl_egl_window,
                                                          width: *mut i32,
                                                          height: *mut i32)
    );

    lazy_static!(
        pub static ref WAYLAND_EGL_OPTION: Option<WaylandEGL> = { 
            WaylandEGL::open("libwayland-egl.so")
        };
        pub static ref WAYLAND_EGL_HANDLE: &'static WaylandEGL = {
            WAYLAND_EGL_OPTION.as_ref().expect("Library libwayland-egl.so could not be loaded.")
        };
    );
}