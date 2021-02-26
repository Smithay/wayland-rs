//! Bindings to the `wayland-cursor.so` library
//!
//! The created handle is named `WAYLAND_CURSOR_HANDLE`.

use crate::client::wl_proxy;
use std::os::raw::{c_char, c_int, c_uint};

pub enum wl_cursor_theme {}

#[repr(C)]
pub struct wl_cursor_image {
    /// actual width
    pub width: u32,
    /// actual height
    pub height: u32,
    /// hot spot x (must be inside image)
    pub hotspot_x: u32,
    /// hot spot y (must be inside image)
    pub hotspot_y: u32,
    /// animation delay to next frame
    pub delay: u32,
}

#[repr(C)]
pub struct wl_cursor {
    pub image_count: c_uint,
    pub images: *mut *mut wl_cursor_image,
    pub name: *mut c_char,
}

external_library!(WaylandCursor, "wayland-cursor",
    functions:
        fn wl_cursor_theme_load(*const c_char, c_int, *mut wl_proxy) -> *mut wl_cursor_theme,
        fn wl_cursor_theme_destroy(*mut wl_cursor_theme) -> (),
        fn wl_cursor_theme_get_cursor(*mut wl_cursor_theme, *const c_char) -> *mut wl_cursor,
        fn wl_cursor_image_get_buffer(*mut wl_cursor_image) -> *mut wl_proxy,
        fn wl_cursor_frame(*mut wl_cursor, u32) -> c_int,
        fn wl_cursor_frame_and_duration(*mut wl_cursor, u32, *mut u32) -> c_int,
);

#[cfg(feature = "dlopen")]
lazy_static::lazy_static!(
    pub static ref WAYLAND_CURSOR_OPTION: Option<WaylandCursor> = {
        // This is a workaround for Ubuntu 17.04, which doesn't have a bare symlink
        // for libwayland-client.so but does have it with the version numbers for
        // whatever reason.
        //
        // We could do some trickery with str slices but that is more trouble
        // than its worth
        let versions = ["libwayland-cursor.so",
                        "libwayland-cursor.so.0"];

        for ver in &versions {
            match unsafe { WaylandCursor::open(ver) } {
                Ok(h) => return Some(h),
                Err(::dlib::DlError::CantOpen(_)) => continue,
                Err(::dlib::DlError::MissingSymbol(s)) => {
                    if ::std::env::var_os("WAYLAND_RS_DEBUG").is_some() {
                        // only print debug messages if WAYLAND_RS_DEBUG is set
                        eprintln!("[wayland-client] Found library {} cannot be used: symbol {} is missing.", ver, s);
                    }
                    return None;
                }
            }
        }
        None
    };
    pub static ref WAYLAND_CURSOR_HANDLE: &'static WaylandCursor = {
        WAYLAND_CURSOR_OPTION.as_ref().expect("Library libwayland-cursor.so could not be loaded.")
    };
);

#[cfg(not(feature = "dlopen"))]
pub fn is_lib_available() -> bool {
    true
}
#[cfg(feature = "dlopen")]
pub fn is_lib_available() -> bool {
    WAYLAND_CURSOR_OPTION.is_some()
}
