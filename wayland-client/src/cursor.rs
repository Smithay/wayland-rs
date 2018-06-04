//! Cursor utilities
//!
//! This module contains bindings to the `libwayland-cursor.so` library.
//!
//! These utilities allows you to laod cursor images in order to match
//! your cursors to the ones of the system.
//!
//! First of all, the function `load_theme` will allow you to load a
//! `CursorTheme`, which represents the full cursor theme.
//!
//! From this theme, you can load a specific `Cursor`, which can each
//! contain several images if the cursor is animated. It provides you
//! with the means of knowing which frame of the animation shoudl be
//! displayed at which time, as well as handles to the buffers containing
//! these frames, to attach them to a wayland surface.

use protocol::wl_buffer::WlBuffer;
use protocol::wl_shm::WlShm;
use std::ffi::{CStr, CString};
use std::marker::PhantomData;
use std::ops::Deref;
use std::os::raw::c_int;
use std::ptr;
use wayland_sys::cursor::*;
use Proxy;

/// Checks if the wayland-cursor lib is available and can be used
///
/// Trying to call any function of this module if the lib cannot
/// be used will result in a panic.
pub fn is_available() -> bool {
    is_lib_available()
}

/// Represents a cursor theme loaded from the system.
pub struct CursorTheme {
    theme: *mut wl_cursor_theme,
}

unsafe impl Send for CursorTheme {}

/// Attempts to load a cursor theme from given name.
///
/// If no name is given or the requested theme is not found, will
/// load the default theme.
///
/// Other arguments are the requested size for the cursor images (ex: 16)
/// and a handle to the global `WlShm` object.
///
/// Panics:
///
/// - If the `wayland-cursor` lib is not available (see `is_available()` function)
///   in this module.
/// - In case of memory allocation failure.
/// - If the theme name provided as argument contains an interior null
pub fn load_theme(name: Option<&str>, size: u32, shm: &Proxy<WlShm>) -> CursorTheme {
    let ptr = if let Some(theme) = name {
        let cstr = CString::new(theme).expect("Theme name contained an interior null.");
        unsafe {
            ffi_dispatch!(
                WAYLAND_CURSOR_HANDLE,
                wl_cursor_theme_load,
                cstr.as_ptr(),
                size as c_int,
                shm.c_ptr()
            )
        }
    } else {
        unsafe {
            ffi_dispatch!(
                WAYLAND_CURSOR_HANDLE,
                wl_cursor_theme_load,
                ptr::null(),
                size as c_int,
                shm.c_ptr()
            )
        }
    };

    assert!(!ptr.is_null(), "Memory allocation failure while loading a theme.");

    CursorTheme { theme: ptr }
}

impl CursorTheme {
    /// Retrieve a cursor from the theme.
    ///
    /// Returns `None` if this cursor is not provided by the theme.
    ///
    /// Panics if the name contains an interior null.
    pub fn get_cursor(&self, name: &str) -> Option<Cursor> {
        let cstr = CString::new(name).expect("Cursor name contained an interior null.");
        let ptr = unsafe {
            ffi_dispatch!(
                WAYLAND_CURSOR_HANDLE,
                wl_cursor_theme_get_cursor,
                self.theme,
                cstr.as_ptr()
            )
        };
        if ptr.is_null() {
            None
        } else {
            Some(Cursor {
                _theme: PhantomData,
                cursor: ptr,
            })
        }
    }
}

impl Drop for CursorTheme {
    fn drop(&mut self) {
        unsafe {
            ffi_dispatch!(WAYLAND_CURSOR_HANDLE, wl_cursor_theme_destroy, self.theme);
        }
    }
}

/// A cursor from a theme. Can contain several images if animated.
pub struct Cursor<'a> {
    _theme: PhantomData<&'a CursorTheme>,
    cursor: *mut wl_cursor,
}

unsafe impl<'a> Send for Cursor<'a> {}

impl<'a> Cursor<'a> {
    /// Retrieve the name of this cursor.
    pub fn name(&self) -> String {
        let name = unsafe { CStr::from_ptr((*self.cursor).name) };
        name.to_string_lossy().into_owned()
    }

    /// Retrieve the number of images contained in this
    /// animated cursor
    pub fn image_count(&self) -> usize {
        let count = unsafe { (*self.cursor).image_count };
        count as usize
    }

    /// Retrieve the image number of cursor animation.
    ///
    /// Returns the image number of the animation that should be displayed
    /// after a given amount of time since the beginning of the animation,
    /// in milliseconds.
    pub fn frame(&self, duration: u32) -> usize {
        let frame = unsafe { ffi_dispatch!(WAYLAND_CURSOR_HANDLE, wl_cursor_frame, self.cursor, duration) };
        frame as usize
    }

    /// Retrieve the image number and its duration.
    ///
    /// Same as `frame()`, but also returns the number of milliseconds this
    /// frame should still be displayed.
    pub fn frame_and_duration(&self, duration: u32) -> (usize, u32) {
        let mut out_duration = 0u32;
        let frame = unsafe {
            ffi_dispatch!(
                WAYLAND_CURSOR_HANDLE,
                wl_cursor_frame_and_duration,
                self.cursor,
                duration,
                &mut out_duration as *mut u32
            )
        } as usize;
        (frame, out_duration)
    }

    /// Retrieve a `CursorImageBuffer` containing the given image of an animation.
    ///
    /// It can be used to be attached to a surface as a classic `WlBuffer`.
    ///
    /// Returns `None` if the frame is out of bounds.
    ///
    /// Note: destroying this buffer (using the `destroy` method) will corrupt
    /// your theme data, so you might not want to do it.
    pub fn frame_buffer(&self, frame: usize) -> Option<CursorImageBuffer> {
        if frame >= self.image_count() {
            None
        } else {
            unsafe {
                let image = *(*self.cursor).images.offset(frame as isize);
                let ptr = ffi_dispatch!(WAYLAND_CURSOR_HANDLE, wl_cursor_image_get_buffer, image);
                let buffer = Proxy::from_c_ptr(ptr);

                Some(CursorImageBuffer {
                    _cursor: PhantomData,
                    buffer: buffer,
                })
            }
        }
    }

    /// Retrive the metadate associated with given frame of the animation.
    ///
    /// The tuple contains: `(width, height, hotspot_x, hotspot_y, delay)`
    ///
    /// Returns `None` if the frame is out of bounds.
    pub fn frame_info(&self, frame: usize) -> Option<(u32, u32, u32, u32, u32)> {
        if frame >= self.image_count() {
            None
        } else {
            let image = unsafe { &**(*self.cursor).images.offset(frame as isize) };
            Some((
                image.width,
                image.height,
                image.hotspot_x,
                image.hotspot_y,
                image.delay,
            ))
        }
    }
}

/// A buffer containing a cursor image.
///
/// You can access the `Proxy<WlBuffer>` via `Deref`.
///
/// Note that this proxy will be considered as "unmanaged" by the crate, as such you should
/// not try to act with it beyong assigning it to `wl_surface`s.
pub struct CursorImageBuffer<'a> {
    _cursor: PhantomData<&'a Cursor<'a>>,
    buffer: Proxy<WlBuffer>,
}

unsafe impl<'a> Send for CursorImageBuffer<'a> {}

impl<'a> Deref for CursorImageBuffer<'a> {
    type Target = Proxy<WlBuffer>;
    fn deref(&self) -> &Proxy<WlBuffer> {
        &self.buffer
    }
}
