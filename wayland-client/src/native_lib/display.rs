use std::io;
use std::os::unix::io::RawFd;
use std::sync::Arc;

use protocol::wl_display::WlDisplay;
use wayland_sys::client::*;

use {ConnectError, Proxy};

use super::EventQueueInner;

pub(crate) struct DisplayInner {
    proxy: WlDisplay,
    display: *mut wl_display,
}

unsafe impl Send for DisplayInner {}
unsafe impl Sync for DisplayInner {}

unsafe fn make_display(ptr: *mut wl_display) -> Result<(Arc<DisplayInner>, EventQueueInner), ConnectError> {
    if ptr.is_null() {
        return Err(ConnectError::NoCompositorListening);
    }

    let display = Arc::new(DisplayInner {
        proxy: Proxy::from_c_ptr(ptr as *mut _).into(),
        display: ptr,
    });

    let evq = EventQueueInner::new(display.clone(), None);

    Ok((display, evq))
}

impl DisplayInner {
    pub unsafe fn from_fd(fd: RawFd) -> Result<(Arc<DisplayInner>, EventQueueInner), ConnectError> {
        if !::wayland_sys::client::is_lib_available() {
            return Err(ConnectError::NoWaylandLib);
        }

        let display_ptr = ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_connect_to_fd, fd);

        make_display(display_ptr)
    }

    pub(crate) fn ptr(&self) -> *mut wl_display {
        self.display
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
            EventQueueInner::new(me.clone(), Some(ptr))
        }
    }

    pub(crate) fn get_proxy(&self) -> &WlDisplay {
        &self.proxy
    }

    pub(crate) fn protocol_error(&self) -> Option<::ProtocolError> {
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
            let interface_name = unsafe { ::std::ffi::CStr::from_ptr((*interface).name) };
            Some(::ProtocolError {
                code,
                object_id: id,
                object_interface: interface_name.to_str().unwrap_or("<unknown>"),
                message: String::new(),
            })
        } else {
            None
        }
    }

    pub(crate) unsafe fn from_external(display_ptr: *mut wl_display) -> (Arc<DisplayInner>, EventQueueInner) {
        let evq_ptr = ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_create_queue, display_ptr);

        let wrapper_ptr = ffi_dispatch!(
            WAYLAND_CLIENT_HANDLE,
            wl_proxy_create_wrapper,
            display_ptr as *mut _
        );
        ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_set_queue, wrapper_ptr, evq_ptr);

        let display = Arc::new(DisplayInner {
            proxy: Proxy::from_c_display_wrapper(wrapper_ptr).into(),
            display: display_ptr,
        });

        let evq = EventQueueInner::new(display.clone(), Some(evq_ptr));
        (display, evq)
    }
}

impl Drop for DisplayInner {
    fn drop(&mut self) {
        if self.proxy.as_ref().c_ptr() == (self.display as *mut _) {
            // disconnect only if we are owning this display
            unsafe {
                ffi_dispatch!(
                    WAYLAND_CLIENT_HANDLE,
                    wl_display_disconnect,
                    self.proxy.as_ref().c_ptr() as *mut wl_display
                );
            }
        }
    }
}
