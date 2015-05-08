use std::io::Error as IoError;
use std::ptr;
use std::sync::{Arc, Mutex};

use libc::{c_void, uint32_t};

use ffi::interfaces::callback::{wl_callback, wl_callback_listener, wl_callback_add_listener};
use ffi::interfaces::display::{wl_display, wl_display_sync};
use ffi::abi;
use ffi::abi::WAYLAND_CLIENT_HANDLE as WCH;

use ffi::FFI;

use super::{From, Registry};

struct InternalDisplay {
    ptr: *mut wl_display,
    listener: Box<DisplayListener>
}

/// InternalDisplay is owning
unsafe impl Send for InternalDisplay {}

/// A wayland Display.
///
/// This is the connexion to the wayland server, it can be cloned, and the
/// connexion is closed once all clones are destroyed.
#[derive(Clone)]
pub struct Display {
    internal: Arc<Mutex<InternalDisplay>>
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
        let internal = self.internal.lock().unwrap();
        unsafe {
            (WCH.wl_display_roundtrip)(internal.ptr);
        }
    }

    /// Dispatches all pending events to their appropriate callbacks.
    ///
    /// Does not block if no events are available.
    pub fn dispatch_pending(&self) {
        let internal = self.internal.lock().unwrap();
        unsafe {
            (WCH.wl_display_dispatch_pending)(internal.ptr);
        }
    }

    /// Dispatches all pending events to their appropriate callbacks.
    ///
    /// If no events are available, blocks until one is received.
    pub fn dispatch(&self) {
        let internal = self.internal.lock().unwrap();
        unsafe {
            (WCH.wl_display_dispatch)(internal.ptr);
        }
    }

    /// Send as much requests to the server as possible.
    ///
    /// Never blocks, but may not send everything. In which case returns
    /// a `WouldBlock` error.
    pub fn flush(&self) -> Result<(), IoError> {
        let internal = self.internal.lock().unwrap();
        if unsafe { (WCH.wl_display_flush)(internal.ptr) } < 0 {
            Err(IoError::last_os_error())
        } else {
            Ok(())
        }
    }

    /// Send a `sync` message to the server.
    ///
    /// It will then anwser with a `done`message. Once the `done` message is received,
    /// the callback provided to `set_sync_callback` will be called.
    ///
    /// As the server processes event sequentially, the callback is thus called once
    /// all pending events have been processed.
    pub fn sync(&self) {
        let internal = self.internal.lock().unwrap();
        unsafe {
            let callback = wl_display_sync(internal.ptr);
            wl_callback_add_listener(
                callback,
                &DONE_LISTENER as *const _,
                &*internal.listener as *const _ as *mut _
            );
        }
    }

    /// Sets the callback of a `done` event.
    pub fn set_sync_callback<F: Fn() + 'static>(&self, f: F) {
        let mut internal = self.internal.lock().unwrap();
        *internal.listener.done.lock().unwrap() = Box::new(f);
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
        self.internal.lock().unwrap().ptr as *const wl_display
    }

    unsafe fn ptr_mut(&self) -> *mut wl_display {
        self.internal.lock().unwrap().ptr
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
                internal: Arc::new(Mutex::new(InternalDisplay{
                    ptr: ptr,
                    listener: Box::new(DisplayListener::default_handlers())
                }))
            })
        }
    }
}

struct DisplayListener {
    /// Handler of the "removed global handler" event
    done: Mutex<Box<Fn()>>,
}

impl DisplayListener {
    fn default_handlers() -> DisplayListener {
        DisplayListener {
            done: Mutex::new(Box::new(move || {}))
        }
    }
}

extern "C" fn display_done(data: *mut c_void, _callback: *mut wl_callback, _data: uint32_t) {
    let listener = unsafe { &*(data as *const DisplayListener) };
    (listener.done.lock().unwrap())();
}

static DONE_LISTENER: wl_callback_listener = wl_callback_listener {
    done: display_done,
};