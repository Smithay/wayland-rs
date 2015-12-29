//! Bindings to the `wayland-cursor.so` library
//!
//! The created handle is named `WAYLAND_CURSOR_HANDLE`.

use {uint32_t, c_uint, c_char, c_int};
use client::wl_proxy;

pub enum wl_cursor_theme { }

#[repr(C)]
pub struct wl_cursor_image {
    /// actual width
    pub width: uint32_t,
    /// actual height
    pub height: uint32_t,
    /// hot spot x (must be inside image)
    pub hotspot_x: uint32_t,
    /// hot spot y (must be inside image)
    pub hotspot_y: uint32_t,
    /// animation delay to next frame
    pub delay: uint32_t
}

#[repr(C)]
pub struct wl_cursor {
    pub image_count: c_uint,
    pub images: *mut *mut wl_cursor_image,
    pub name: *mut c_char
}

external_library!(WaylandCursor, "wayland-cursor",
    functions:
        fn wl_cursor_theme_load(*const c_char, c_int, *mut wl_proxy) -> *mut wl_cursor_theme,
        fn wl_cursor_theme_destroy(*mut wl_cursor_theme) -> (),
        fn wl_cursor_theme_get_cursor(*mut wl_cursor_theme, *const c_char) -> *mut wl_cursor,
        fn wl_cursor_image_get_buffer(*mut wl_cursor_image) -> *mut wl_proxy,
        fn wl_cursor_frame(*mut wl_cursor, uint32_t) -> c_int,
        fn wl_cursor_frame_and_duration(*mut wl_cursor, uint32_t, *mut uint32_t) -> c_int
);

#[cfg(feature = "dlopen")]
lazy_static!(
    pub static ref WAYLAND_CURSOR_OPTION: Option<WaylandCursor> = {
        WaylandCursor::open("libwayland-cursor.so").ok()
    };
    pub static ref WAYLAND_CURSOR_HANDLE: &'static WaylandCursor = {
        WAYLAND_CURSOR_OPTION.as_ref().expect("Library libwayland-cursor.so could not be loaded.")
    };
);

#[cfg(not(feature = "dlopen"))]
pub fn is_lib_available() -> bool { true }
#[cfg(feature = "dlopen")]
pub fn is_lib_available() -> bool { WAYLAND_CURSOR_OPTION.is_some() }