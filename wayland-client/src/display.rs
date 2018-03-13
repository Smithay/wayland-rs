#[cfg(feature = "native_lib")]
use std::ffi::{CString, OsString};
#[cfg(feature = "native_lib")]
use std::os::unix::ffi::OsStringExt;
use std::sync::Arc;
use std::ops::Deref;

use Proxy;
use EventQueue;

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
    InvalidName,
}

pub(crate) struct DisplayInner {
    proxy: Proxy<::protocol::wl_display::WlDisplay>,
}

impl DisplayInner {
    #[cfg(feature = "native_lib")]
    pub(crate) fn ptr(&self) -> *mut wl_display {
        self.proxy.c_ptr() as *mut _
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
                ffi_dispatch!(
                    WAYLAND_CLIENT_HANDLE,
                    wl_display_disconnect,
                    self.proxy.c_ptr() as *mut wl_display
                );
            }
        }
    }
}

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
            }),
        };

        let evq = EventQueue::new(display.inner.clone(), None);

        Ok((display, evq))
    }

    pub fn connect_to_env() -> Result<(Display, EventQueue), ConnectError> {
        #[cfg(not(feature = "native_lib"))]
        {
            unimplemented!()
        }
        #[cfg(feature = "native_lib")]
        {
            unsafe {
                let display_ptr = ffi_dispatch!(
                    WAYLAND_CLIENT_HANDLE,
                    wl_display_connect,
                    ::std::ptr::null()
                );

                Display::make_display(display_ptr)
            }
        }
    }

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

    pub fn create_event_queue(&self) -> EventQueue {
        #[cfg(not(feature = "native_lib"))]
        {
            unimplemented!()
        }
        #[cfg(feature = "native_lib")]
        unsafe {
            let ptr = ffi_dispatch!(
                WAYLAND_CLIENT_HANDLE,
                wl_display_create_queue,
                self.inner.ptr()
            );

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
