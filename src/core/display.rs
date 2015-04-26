use std::io::Error as IoError;
use std::ptr;

use ffi::interfaces::display::wl_display;
use ffi::abi;

use ffi::FFI;

use super::{From, Registry};

/// A wayland Display.
///
/// This is the connexion to the wayland server, once it
/// goes out of scope the connexion will be closed.
pub struct Display {
    ptr: *mut wl_display
}

impl Display {
    /// Creates a Registry associated to this Display and returns it.
    pub fn get_registry<'a>(&'a self) -> Registry<'a> {
        From::from(self)
    }

    /// Performs a blocking synchronisation of the events of the server.
    ///
    /// This call will block until the wayland server has processed all
    /// the queries from this Display instance.
    pub fn sync_roundtrip(&self) {
        unsafe {
            abi::wl_display_roundtrip(self.ptr);
        }
    }

    /// Dispatches all pending events to their appropriate callbacks.
    ///
    /// Does not block if no events are available.
    pub fn dispatch_pending(&self) {
        unsafe {
            abi::wl_display_dispatch_pending(self.ptr);
        }
    }

    /// Dispatches all pending events to their appropriate callbacks.
    ///
    /// If no events are available, blocks until one is received.
    pub fn dispatch(&self) {
        unsafe {
            abi::wl_display_dispatch(self.ptr);
        }
    }

    /// Send as much requests to the server as possible.
    ///
    /// Never blocks, but may not send everything. In which case returns
    /// a `WouldBlock` error.
    pub fn flush(&self) -> Result<(), IoError> {
        if unsafe { abi::wl_display_flush(self.ptr) } < 0 {
            Err(IoError::last_os_error())
        } else {
            Ok(())
        }
    }
}

impl Drop for Display {
    fn drop(&mut self) {
        unsafe {
            abi::wl_display_disconnect(self.ptr);
        }
    }
}

impl FFI for Display {
    type Ptr = wl_display;

    fn ptr(&self) -> *const wl_display {
        self.ptr as *const wl_display
    }

    unsafe fn ptr_mut(&self) -> *mut wl_display {
        self.ptr
    }
}

/// Tries to connect to the default wayland display.
///
/// If the `WAYLAND_DISPLAY` environment variable is set, it will
/// be used. Otherwise it defaults to `"wayland-0"`.
pub fn default_display() -> Option<Display> {
    unsafe {
        let ptr = abi::wl_display_connect(ptr::null_mut());
        if ptr.is_null() {
            None
        } else {
            Some(Display {
                ptr: ptr
            })
        }
    }
}