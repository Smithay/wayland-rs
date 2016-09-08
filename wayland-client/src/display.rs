use std::io;
use std::ffi::CStr;

use wayland_sys::client::*;

use event_queue::{create_event_queue, EventQueue};
use generated::client::wl_display::{WlDisplay, Error as DisplayError};
use Proxy;

/// Enum representing the possible reasons why connecting to the wayland server failed
#[derive(Debug)]
pub enum ConnectError {
    /// The library was compiled with the `dlopen` feature, and the `libwayland-client.so`
    /// library could not be found at runtime
    NoWaylandLib,
    /// Any needed library was found, but the listening socket of the server could not be
    /// found.
    ///
    /// Most of the time, this means that the program was not started from a wayland session.
    NoCompositorListening
}

/// Enum representing possible errors fatal to a wayland session
#[derive(Debug)]
pub enum FatalError {
    /// Session aborted after an I/O error
    Io(io::Error),
    /// Session aborted after a protocol error
    Protocol { kind: DisplayError, interface: String, id: u32 }
}

/// Connect to the compositor socket
///
/// Attempt to connect to a Wayland compositor according to the environment variables.
///
/// On success, returns the display object, as well as the default event iterator associated with it.
pub fn default_connect() -> Result<(WlDisplay, EventQueue), ConnectError> {
    if !::wayland_sys::client::is_lib_available() { return Err(ConnectError::NoWaylandLib) }
    let ptr = unsafe { ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_connect, ::std::ptr::null()) };
    if ptr.is_null() {
        Err(ConnectError::NoCompositorListening)
    } else {
        let display = unsafe { WlDisplay::from_ptr(ptr as *mut _) };
        let eventiter = unsafe { create_event_queue(display.ptr() as *mut wl_display, None) };
        Ok((display, eventiter))
    }
}

impl WlDisplay {
    /// Non-blocking write to the server
    ///
    /// Will write as many pending requests as possible to the server socket. Never blocks: if not all
    /// requests coul be written, will return an io error `WouldBlock`.
    ///
    /// On success returns the number of written requests.
    pub fn flush(&self) -> io::Result<i32> {
        let ret = unsafe { ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_flush, self.ptr() as *mut _) };
        if ret >= 0 { Ok(ret) } else { Err(io::Error::last_os_error()) }
    }

    /// Create a new EventQueue
    ///
    /// No object is by default attached to it.
    pub fn create_event_queue(&self) -> EventQueue {
        let evq = unsafe { ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_create_queue, self.ptr() as *mut _) };
        unsafe { create_event_queue(self.ptr() as *mut _, Some(evq)) }
    }

    /// Get the last error that occured on the session
    ///
    /// Such errors are *fatal*, meaning that if this function does not
    /// return `None`, your session is not usable any longer.
    ///
    /// As such, this function mostly provide diagnistics information. You can have a hint
    /// an error might have been generated if I/O methods of EventQueue start returning errors.
    pub fn last_error(&self) -> Option<FatalError> {
        let err = unsafe { ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_get_error, self.ptr() as *mut _) };
        if err == 0 {
            None
        } else if err == ::libc::EPROTO {
            let mut interface = ::std::ptr::null_mut();
            let mut id = 0;
            let code = unsafe { ffi_dispatch!(
                WAYLAND_CLIENT_HANDLE, wl_display_get_protocol_error,
                self.ptr() as *mut _, &mut interface, &mut id
            ) };
            let interface = if interface.is_null() {
                "<unkown interface>".to_owned()
            } else {
                unsafe { CStr::from_ptr((*interface).name) }.to_string_lossy().into_owned()
            };
            Some(FatalError::Protocol {
                kind: DisplayError::from_raw(code).expect("Unknown protocol error"),
                interface: interface,
                id: id
            })
        } else {
            Some(FatalError::Io(io::Error::from_raw_os_error(err)))
        }
    }
}

impl Drop for WlDisplay {
    fn drop(&mut self) {
        unsafe { ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_disconnect, self.ptr() as *mut _) }
    }
}
