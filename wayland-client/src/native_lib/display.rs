use std::ffi::{CString, OsString};
use std::io;
use std::os::unix::ffi::OsStringExt;
use std::ptr;
use std::sync::Arc;

use protocol::wl_display::WlDisplay;
use wayland_sys::client::*;

use {ConnectError, Proxy};

use super::EventQueueInner;

pub(crate) struct DisplayInner {
    proxy: Proxy<WlDisplay>,
    display: *mut wl_display
}

unsafe fn make_display(ptr: *mut wl_display) -> Result<(Arc<DisplayInner>, EventQueueInner), ConnectError> {
    if ptr.is_null() {
        return Err(ConnectError::NoCompositorListening);
    }

    let display = Arc::new(DisplayInner {
        proxy: Proxy::from_c_ptr(ptr as *mut _),
        display: ptr,
    });

    let evq = EventQueueInner::new(display.clone(), None);

    Ok((display, evq))
}

impl DisplayInner {
    pub fn connect_to_name(
        name: Option<OsString>,
    ) -> Result<(Arc<DisplayInner>, EventQueueInner), ConnectError> {
        if !::wayland_sys::client::is_lib_available() {
            return Err(ConnectError::NoWaylandLib);
        }

        let name = match name {
            Some(s) => Some(CString::new(s.into_vec()).map_err(|_| ConnectError::InvalidName)?),
            None => None,
        };

        unsafe {
            let display_ptr = ffi_dispatch!(
                WAYLAND_CLIENT_HANDLE,
                wl_display_connect,
                name.as_ref().map(|s| s.as_ptr()).unwrap_or(ptr::null())
            );

            make_display(display_ptr)
        }
    }

    pub(crate) fn ptr(&self) -> *mut wl_display {
        self.display
    }

    pub(crate) fn flush(&self) -> io::Result<i32> {
        let ret = unsafe { ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_flush, self.ptr()) };
        if ret >= 0 {
            Ok(ret)
        } else {
            Err(io::Error::last_os_error())
        }
    }

    pub(crate) fn create_event_queue(me: &Arc<DisplayInner>) -> EventQueueInner {
        unsafe {
            let ptr = ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_create_queue, me.ptr());
            EventQueueInner::new(me.clone(), Some(ptr))
        }
    }

    pub(crate) fn get_proxy(&self) -> &Proxy<WlDisplay> {
        &self.proxy
    }

    pub(crate) unsafe fn from_external(display_ptr: *mut wl_display) -> (Arc<DisplayInner>, EventQueueInner) {
        let evq_ptr = ffi_dispatch!(
            WAYLAND_CLIENT_HANDLE,
            wl_display_create_queue,
            display_ptr
        );

        let wrapper_ptr = ffi_dispatch!(
            WAYLAND_CLIENT_HANDLE,
            wl_proxy_create_wrapper,
            display_ptr as *mut _
        );
        ffi_dispatch!(
            WAYLAND_CLIENT_HANDLE,
            wl_proxy_set_queue,
            wrapper_ptr,
            evq_ptr
        );

        let display = Arc::new(DisplayInner {
            proxy: Proxy::from_c_display_wrapper(wrapper_ptr),
            display: display_ptr
        });

        let evq = EventQueueInner::new(display.clone(), Some(evq_ptr));
        (display, evq)
    }
}

impl Drop for DisplayInner {
    fn drop(&mut self) {
        if self.proxy.c_ptr() == (self.display as *mut _) {
            // disconnect only if we are owning this display
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
