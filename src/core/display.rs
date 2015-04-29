use std::io::Error as IoError;
use std::ptr;
use std::rc::Rc;

use ffi::interfaces::display::wl_display;
use ffi::abi;
use ffi::abi::WAYLAND_CLIENT_HANDLE as WCH;

use ffi::FFI;

use super::{From, Registry};

struct InternalDisplay {
    ptr: *mut wl_display
}

/// A wayland Display.
///
/// This is the connexion to the wayland server, it can be cloned, and the
/// connexion is closed once all clones are destroyed.
#[derive(Clone)]
pub struct Display {
    internal: Rc<InternalDisplay>
}

impl Display {
    /// Creates a Registry associated to this Display and returns it.
    ///
    /// The registry holds a clone of the Display, and thus will maintain the
    /// connexion alive.
    pub fn get_registry(&self) -> Registry {
        From::from(self.clone())
    }

    /// Performs a blocking synchronisation of the events of the server.
    ///
    /// This call will block until the wayland server has processed all
    /// the queries from this Display instance.
    pub fn sync_roundtrip(&self) {
        unsafe {
            (WCH.wl_display_roundtrip)(self.internal.ptr);
        }
    }

    /// Dispatches all pending events to their appropriate callbacks.
    ///
    /// Does not block if no events are available.
    pub fn dispatch_pending(&self) {
        unsafe {
            (WCH.wl_display_dispatch_pending)(self.internal.ptr);
        }
    }

    /// Dispatches all pending events to their appropriate callbacks.
    ///
    /// If no events are available, blocks until one is received.
    pub fn dispatch(&self) {
        unsafe {
            (WCH.wl_display_dispatch)(self.internal.ptr);
        }
    }

    /// Send as much requests to the server as possible.
    ///
    /// Never blocks, but may not send everything. In which case returns
    /// a `WouldBlock` error.
    pub fn flush(&self) -> Result<(), IoError> {
        if unsafe { (WCH.wl_display_flush)(self.internal.ptr) } < 0 {
            Err(IoError::last_os_error())
        } else {
            Ok(())
        }
    }
}

impl Drop for InternalDisplay {
    fn drop(&mut self) {
        unsafe {
            (WCH.wl_display_disconnect)(self.ptr);
        }
    }
}

impl FFI for Display {
    type Ptr = wl_display;

    fn ptr(&self) -> *const wl_display {
        self.internal.ptr as *const wl_display
    }

    unsafe fn ptr_mut(&self) -> *mut wl_display {
        self.internal.ptr
    }
}

/// Tries to connect to the default wayland display.
///
/// If the `WAYLAND_DISPLAY` environment variable is set, it will
/// be used. Otherwise it defaults to `"wayland-0"`.
///
/// Will return `None` if either:
///
/// - the library `libwayland-client.so` is not available
/// - the connexion to the wayland server could not be done.
pub fn default_display() -> Option<Display> {
    unsafe {
        let handle = match abi::WAYLAND_CLIENT_OPTION.as_ref() {
            Some(h) => h,
            None => return None
        };
        let ptr = (handle.wl_display_connect)(ptr::null_mut());
        if ptr.is_null() {
            None
        } else {
            Some(Display {
                internal: Rc::new(InternalDisplay{ ptr: ptr})
            })
        }
    }
}