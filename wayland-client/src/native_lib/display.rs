use std::io;
use std::os::unix::io::RawFd;
use std::sync::Arc;

use crate::protocol::wl_display::WlDisplay;
use wayland_sys::client::*;

use crate::{ConnectError, Proxy};

use super::{EventQueueInner, ProxyInner};

pub(crate) struct DisplayInner {
    proxy: Proxy<WlDisplay>,
    display: Arc<DisplayGuard>,
}

pub(crate) struct DisplayGuard {
    ptr: *mut wl_display,
    external: bool,
}

unsafe impl Send for DisplayInner {}
unsafe impl Sync for DisplayInner {}

unsafe fn make_display(ptr: *mut wl_display) -> Result<Arc<DisplayInner>, ConnectError> {
    if ptr.is_null() {
        return Err(ConnectError::NoCompositorListening);
    }

    let mut inner = DisplayInner {
        proxy: Proxy::from_c_ptr(ptr as *mut _),
        display: Arc::new(DisplayGuard { ptr, external: false }),
    };

    inner.proxy.inner.display = Some(Arc::downgrade(&inner.display));

    Ok(Arc::new(inner))
}

impl DisplayInner {
    pub unsafe fn from_fd(fd: RawFd) -> Result<Arc<DisplayInner>, ConnectError> {
        if !::wayland_sys::client::is_lib_available() {
            return Err(ConnectError::NoWaylandLib);
        }

        let display_ptr = ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_connect_to_fd, fd);

        make_display(display_ptr)
    }

    pub(crate) fn get_connection_fd(&self) -> ::std::os::unix::io::RawFd {
        unsafe { ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_get_fd, self.ptr()) }
    }

    pub(crate) fn ptr(&self) -> *mut wl_display {
        self.display.ptr
    }

    pub(crate) fn flush(&self) -> io::Result<()> {
        let ret = unsafe { ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_flush, self.ptr()) };
        if ret >= 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }

    pub(crate) fn create_event_queue(me: &Arc<DisplayInner>) -> EventQueueInner {
        unsafe {
            let ptr = ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_create_queue, me.ptr());
            EventQueueInner::new(me.clone(), ptr)
        }
    }

    pub(crate) fn get_proxy(&self) -> &Proxy<WlDisplay> {
        &self.proxy
    }

    pub(crate) fn protocol_error(&self) -> Option<crate::ProtocolError> {
        let ret = unsafe { ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_get_error, self.ptr()) };
        if ret == ::nix::errno::Errno::EPROTO as i32 {
            let mut interface = ::std::ptr::null_mut();
            let mut id = 0;
            let code = unsafe {
                ffi_dispatch!(
                    WAYLAND_CLIENT_HANDLE,
                    wl_display_get_protocol_error,
                    self.ptr(),
                    &mut interface,
                    &mut id
                )
            };
            let interface_name = if !interface.is_null() {
                unsafe { ::std::ffi::CStr::from_ptr((*interface).name) }
                    .to_str()
                    .unwrap_or("<unknown>")
            } else {
                "<unknown>"
            };
            Some(crate::ProtocolError {
                code,
                object_id: id,
                object_interface: interface_name,
                message: String::new(),
            })
        } else {
            None
        }
    }

    pub(crate) unsafe fn from_external(display_ptr: *mut wl_display) -> Arc<DisplayInner> {
        Arc::new(DisplayInner {
            proxy: Proxy::wrap(ProxyInner::from_external_display(display_ptr as *mut _)),
            display: Arc::new(DisplayGuard { ptr: display_ptr, external: true }),
        })
    }
}

impl Drop for DisplayGuard {
    fn drop(&mut self) {
        if !self.external {
            // disconnect only if we are owning this display
            unsafe {
                ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_disconnect, self.ptr);
            }
        }
    }
}
