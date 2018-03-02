use std::io::{Error as IoError, Result as IoResult};
use std::sync::Arc;

use display::DisplayInner;

#[cfg(feature = "native_lib")]
use wayland_sys::client::*;

pub struct EventQueue {
    wlevq: Option<*mut wl_event_queue>,
    inner: Arc<DisplayInner>,
}

impl EventQueue {
    pub(crate) unsafe fn new(inner: Arc<DisplayInner>, evq: Option<*mut wl_event_queue>) -> EventQueue {
        EventQueue {
            inner: inner,
            wlevq: evq,
        }
    }

    /// Dispatches events from the internal buffer.
    ///
    /// Dispatches all events to their appropriaters.
    /// If not events were in the internal buffer, will block until
    /// some events are read and dispatch them.
    /// This process can insert events in the internal buffers of
    /// other event queues.
    ///
    /// If an error is returned, your connection with the wayland
    /// compositor is probably lost.
    pub fn dispatch(&mut self) -> IoResult<u32> {
        #[cfg(not(feature = "native_lib"))]
        {}
        #[cfg(feature = "native_lib")]
        {
            let ret = match self.wlevq {
                Some(evq) => unsafe {
                    ffi_dispatch!(
                        WAYLAND_CLIENT_HANDLE,
                        wl_display_dispatch_queue,
                        self.inner.ptr(),
                        evq
                    )
                },
                None => unsafe {
                    ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_dispatch, self.inner.ptr())
                },
            };
            if ret >= 0 {
                Ok(ret as u32)
            } else {
                Err(IoError::last_os_error())
            }
        }
    }

    /// Dispatches pending events from the internal buffer.
    ///
    /// Dispatches all events to their appropriaters.
    /// Never blocks, if not events were pending, simply returns
    /// `Ok(0)`.
    ///
    /// If an error is returned, your connection with the wayland
    /// compositor is probably lost.
    pub fn dispatch_pending(&mut self) -> IoResult<u32> {
        #[cfg(not(feature = "native_lib"))]
        {}
        #[cfg(feature = "native_lib")]
        {
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
                Err(IoError::last_os_error())
            }
        }
    }

    /// Synchronous roundtrip
    ///
    /// This call will cause a synchonous roundtrip with the wayland server. It will block until all
    /// pending requests of this queue are sent to the server and it has processed all of them and
    /// send the appropriate events.
    ///
    /// Handlers are called as a consequence.
    ///
    /// On success returns the number of dispatched events.
    pub fn sync_roundtrip(&mut self) -> IoResult<i32> {
        #[cfg(not(feature = "native_lib"))]
        {}
        #[cfg(feature = "native_lib")]
        {
            let ret = unsafe {
                match self.wlevq {
                    Some(evtq) => ffi_dispatch!(
                        WAYLAND_CLIENT_HANDLE,
                        wl_display_roundtrip_queue,
                        self.inner.ptr(),
                        evtq
                    ),
                    None => ffi_dispatch!(
                        WAYLAND_CLIENT_HANDLE,
                        wl_display_roundtrip,
                        self.inner.ptr()
                    ),
                }
            };
            if ret >= 0 {
                Ok(ret)
            } else {
                Err(IoError::last_os_error())
            }
        }
    }
}

impl Drop for EventQueue {
    fn drop(&mut self) {
        #[cfg(feature = "nativel_lib")]
        {
            if let Some(evq) = self.wlevq {
                unsafe {
                    ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_event_queue_destroy, evq);
                }
            }
        }
    }
}
