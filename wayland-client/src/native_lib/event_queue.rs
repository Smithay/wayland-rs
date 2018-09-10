use std::io;
use std::ptr;
use std::sync::Arc;

use wayland_sys::client::*;

use super::DisplayInner;

pub(crate) struct EventQueueInner {
    wlevq: Option<*mut wl_event_queue>,
    inner: Arc<super::DisplayInner>,
}

impl EventQueueInner {
    pub(crate) fn new(inner: Arc<DisplayInner>, wlevq: Option<*mut wl_event_queue>) -> EventQueueInner {
        EventQueueInner { inner, wlevq }
    }

    #[cfg(feature = "eventloop")]
    pub(crate) fn get_connection_fd(&self) -> ::std::os::unix::io::RawFd {
        unsafe { ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_get_fd, self.inner.ptr()) }
    }

    pub fn dispatch(&self) -> io::Result<u32> {
        let ret = match self.wlevq {
            Some(evq) => unsafe {
                ffi_dispatch!(
                    WAYLAND_CLIENT_HANDLE,
                    wl_display_dispatch_queue,
                    self.inner.ptr(),
                    evq
                )
            },
            None => unsafe { ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_dispatch, self.inner.ptr()) },
        };
        if ret >= 0 {
            Ok(ret as u32)
        } else {
            Err(io::Error::last_os_error())
        }
    }

    pub fn dispatch_pending(&self) -> io::Result<u32> {
        let ret = match self.wlevq {
            Some(evq) => unsafe {
                ffi_dispatch!(
                    WAYLAND_CLIENT_HANDLE,
                    wl_display_dispatch_queue_pending,
                    self.inner.ptr(),
                    evq
                )
            },
            None => unsafe {
                ffi_dispatch!(
                    WAYLAND_CLIENT_HANDLE,
                    wl_display_dispatch_pending,
                    self.inner.ptr()
                )
            },
        };
        if ret >= 0 {
            Ok(ret as u32)
        } else {
            Err(io::Error::last_os_error())
        }
    }

    pub fn sync_roundtrip(&self) -> io::Result<u32> {
        let ret = unsafe {
            match self.wlevq {
                Some(evtq) => ffi_dispatch!(
                    WAYLAND_CLIENT_HANDLE,
                    wl_display_roundtrip_queue,
                    self.inner.ptr(),
                    evtq
                ),
                None => ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_roundtrip, self.inner.ptr()),
            }
        };
        if ret >= 0 {
            Ok(ret as u32)
        } else {
            Err(io::Error::last_os_error())
        }
    }

    pub(crate) fn prepare_read(&self) -> Result<(), ()> {
        let ret = unsafe {
            match self.wlevq {
                Some(evtq) => ffi_dispatch!(
                    WAYLAND_CLIENT_HANDLE,
                    wl_display_prepare_read_queue,
                    self.inner.ptr(),
                    evtq
                ),
                None => ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_prepare_read, self.inner.ptr()),
            }
        };
        if ret >= 0 {
            Ok(())
        } else {
            Err(())
        }
    }

    pub(crate) fn read_events(&self) -> io::Result<i32> {
        let ret = unsafe { ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_read_events, self.inner.ptr()) };
        if ret >= 0 {
            Ok(ret)
        } else {
            Err(io::Error::last_os_error())
        }
    }

    pub(crate) fn cancel_read(&self) {
        unsafe { ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_cancel_read, self.inner.ptr()) }
    }

    pub(crate) unsafe fn assign_proxy(&self, proxy: *mut wl_proxy) {
        ffi_dispatch!(
            WAYLAND_CLIENT_HANDLE,
            wl_proxy_set_queue,
            proxy,
            self.wlevq.unwrap_or(ptr::null_mut())
        )
    }
}

impl Drop for EventQueueInner {
    fn drop(&mut self) {
        if let Some(evq) = self.wlevq {
            unsafe {
                ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_event_queue_destroy, evq);
            }
        }
    }
}
