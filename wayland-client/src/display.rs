#[cfg(feature = "native_lib")]
use std::ffi::{CString, OsString};
use std::io;
use std::ops::Deref;
#[cfg(feature = "native_lib")]
use std::os::unix::ffi::OsStringExt;
use std::sync::Arc;

use EventQueue;
use Proxy;

#[cfg(feature = "native_lib")]
use wayland_sys::client::*;

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
    NoCompositorListening,
    /// The provided socket name is invalid
    InvalidName,
}

pub(crate) struct DisplayInner {
    proxy: Proxy<::protocol::wl_display::WlDisplay>,
    display: *mut wl_display,
}

unsafe impl Send for DisplayInner {}
unsafe impl Sync for DisplayInner {}

impl DisplayInner {
    #[cfg(feature = "native_lib")]
    pub(crate) fn ptr(&self) -> *mut wl_display {
        self.display
    }
}

impl Drop for DisplayInner {
    fn drop(&mut self) {
        #[cfg(not(feature = "native_lib"))]
        {
            unimplemented!();
        }
        #[cfg(feature = "native_lib")]
        {
            unsafe {
                if self.proxy.c_ptr() == (self.display as *mut _) {
                    // disconnect only if we are owning this display
                    ffi_dispatch!(
                        WAYLAND_CLIENT_HANDLE,
                        wl_display_disconnect,
                        self.proxy.c_ptr() as *mut wl_display
                    );
                }
            }
        }
    }
}

/// A connection to a wayland server
///
/// This object both represent the connection to the server, and as such
/// must be kept alive as long as you are connected, and contains the
/// primary `WlDisplay` wayland object, from which you can create all
/// your need objects. The inner `Proxy<WlDisplay>` can be accessed via
/// `Deref`.
pub struct Display {
    inner: Arc<DisplayInner>,
}

impl Display {
    #[cfg(feature = "native_lib")]
    unsafe fn make_display(ptr: *mut wl_display) -> Result<(Display, EventQueue), ConnectError> {
        if ptr.is_null() {
            return Err(ConnectError::NoCompositorListening);
        }

        let display = Display {
            inner: Arc::new(DisplayInner {
                proxy: Proxy::from_display(ptr),
                display: ptr,
            }),
        };

        let evq = EventQueue::new(display.inner.clone(), None);

        Ok((display, evq))
    }

    #[cfg(feature = "native_lib")]
    /// Create a Display and Event Queue from an external display
    ///
    /// This allows you to interface with an already-existing wayland connection,
    /// for example provided by a GUI toolkit.
    ///
    /// To avoid interferences with the owner of the connection, wayland-client will
    /// create a new event queue and register a wrapper of the `wl_display` to this queue,
    /// then provide them to you. You can then use them as if they came from a direct
    /// wayland connection.
    ///
    /// Note that if you need to retrieve the actual `wl_display` back (rather than its wrapper),
    /// you must use the `get_display_ptr()` method.
    pub unsafe fn from_external_display(display_ptr: *mut wl_display) -> (Display, EventQueue) {
        let evq_ptr = ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_create_queue, display_ptr);

        let wrapper_ptr = ffi_dispatch!(
            WAYLAND_CLIENT_HANDLE,
            wl_proxy_create_wrapper,
            display_ptr as *mut _
        );
        ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_set_queue, wrapper_ptr, evq_ptr);

        let display = Display {
            inner: Arc::new(DisplayInner {
                proxy: Proxy::from_display_wrapper(wrapper_ptr),
                display: display_ptr,
            }),
        };

        let evq = EventQueue::new(display.inner.clone(), Some(evq_ptr));
        (display, evq)
    }

    #[cfg(feature = "native_lib")]
    /// Retrieve the `wl_display` pointer
    ///
    /// If this `Display` was created from an external `wl_display`, its `c_ptr()` method will
    /// return a wrapper to the actual display. While this is perfectly good as a `wl_proxy`
    /// pointer, to send requests, this is not the actual `wl_display` and cannot be used as such.
    ///
    /// This method will give you the `wl_display`.
    pub fn get_display_ptr(&self) -> *mut wl_display {
        self.inner.ptr()
    }

    /// Attempt to connect to a wayland server using the contents of the environment variables
    ///
    /// If the `WAYLAND_DISPLAY` variable is set, it will try to connect to the socket it points
    /// to. Otherwise, it will default to `wayland-0`.
    ///
    /// On success, you are given the `Display` object as well as the main `EventQueue` hosting
    /// the `WlDisplay` wayland object.
    ///
    /// This requires the `XDG_RUNTIME_DIR` variable to be properly set.
    pub fn connect_to_env() -> Result<(Display, EventQueue), ConnectError> {
        #[cfg(not(feature = "native_lib"))]
        {
            unimplemented!()
        }
        #[cfg(feature = "native_lib")]
        {
            if !::wayland_sys::client::is_lib_available() {
                return Err(ConnectError::NoWaylandLib);
            }

            unsafe {
                let display_ptr =
                    ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_connect, ::std::ptr::null());

                Display::make_display(display_ptr)
            }
        }
    }

    /// Attempt to connect to a wayland server socket with given name
    ///
    /// On success, you are given the `Display` object as well as the main `EventQueue` hosting
    /// the `WlDisplay` wayland object.
    ///
    /// This requires the `XDG_RUNTIME_DIR` variable to be properly set.
    pub fn connect_to_name<S: Into<OsString>>(name: S) -> Result<(Display, EventQueue), ConnectError> {
        #[cfg(not(feature = "native_lib"))]
        {
            unimplemented!()
        }
        #[cfg(feature = "native_lib")]
        {
            if !::wayland_sys::client::is_lib_available() {
                return Err(ConnectError::NoWaylandLib);
            }

            let name = CString::new(name.into().into_vec()).map_err(|_| ConnectError::InvalidName)?;

            unsafe {
                let display_ptr = ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_connect, name.as_ptr());

                Display::make_display(display_ptr)
            }
        }
    }

    /// Non-blocking write to the server
    ///
    /// Outgoing messages to the server are buffered by the library for efficiency. This method
    /// flushes the internal buffer to the server socket.
    ///
    /// Will write as many pending requests as possible to the server socket. Never blocks: if not all
    /// requests coul be written, will return an io error `WouldBlock`.
    ///
    /// On success returns the number of written requests.
    pub fn flush(&self) -> io::Result<i32> {
        let ret = unsafe { ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_flush, self.inner.ptr()) };
        if ret >= 0 {
            Ok(ret)
        } else {
            Err(io::Error::last_os_error())
        }
    }

    /// Create a new event queue associated with this wayland connection
    pub fn create_event_queue(&self) -> EventQueue {
        #[cfg(not(feature = "native_lib"))]
        {
            unimplemented!()
        }
        #[cfg(feature = "native_lib")]
        unsafe {
            let ptr = ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_create_queue, self.inner.ptr());

            EventQueue::new(self.inner.clone(), Some(ptr))
        }
    }
}

impl Deref for Display {
    type Target = Proxy<::protocol::wl_display::WlDisplay>;
    fn deref(&self) -> &Proxy<::protocol::wl_display::WlDisplay> {
        &self.inner.proxy
    }
}
