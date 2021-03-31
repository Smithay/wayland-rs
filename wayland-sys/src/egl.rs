//! Bindings to the EGL library `libwayland-egl.so`
//!
//! This lib allows to create EGL surfaces out of wayland surfaces.
//!
//! The created handle is named `WAYLAND_EGl_HANDLE`.

use crate::client::wl_proxy;
use std::os::raw::c_int;

pub enum wl_egl_window {}

external_library!(WaylandEgl, "wayland-egl",
    functions:
        fn wl_egl_window_create(*mut wl_proxy, c_int, c_int) -> *mut wl_egl_window,
        fn wl_egl_window_destroy(*mut wl_egl_window) -> (),
        fn wl_egl_window_resize(*mut wl_egl_window, c_int, c_int, c_int, c_int) -> (),
        fn wl_egl_window_get_attached_size(*mut wl_egl_window, *mut c_int, *mut c_int) -> (),
);

#[cfg(feature = "dlopen")]
lazy_static::lazy_static!(
    pub static ref WAYLAND_EGL_OPTION: Option<WaylandEgl> = {
        // This is a workaround for Ubuntu 17.04, which doesn't have a bare symlink
        // for libwayland-client.so but does have it with the version numbers for
        // whatever reason.
        //
        // We could do some trickery with str slices but that is more trouble
        // than its worth
        let versions = ["libwayland-egl.so",
                        "libwayland-egl.so.1"];

        for ver in &versions {
            match unsafe { WaylandEgl::open(ver) } {
                Ok(h) => return Some(h),
                Err(::dlib::DlError::CantOpen(_)) => continue,
                Err(::dlib::DlError::MissingSymbol(s)) => {
                    log::error!("Found library {} cannot be used: symbol {} is missing.", ver, s);
                    return None;
                }
            }
        }
        None
    };
    pub static ref WAYLAND_EGL_HANDLE: &'static WaylandEgl = {
        WAYLAND_EGL_OPTION.as_ref().expect("Library libwayland-egl.so could not be loaded.")
    };
);

#[cfg(not(feature = "dlopen"))]
pub fn is_lib_available() -> bool {
    true
}
#[cfg(feature = "dlopen")]
pub fn is_lib_available() -> bool {
    WAYLAND_EGL_OPTION.is_some()
}
