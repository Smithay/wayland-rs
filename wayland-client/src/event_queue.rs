use std::io::{Error as IoError, Result as IoResult};
use std::ptr;
use std::rc::Rc;
use std::sync::Arc;

use display::DisplayInner;

#[cfg(feature = "native_lib")]
use wayland_sys::client::*;

struct EventQueueInner {
    #[cfg(feature = "native_lib")] wlevq: Option<*mut wl_event_queue>,
    inner: Arc<DisplayInner>,
}

/// An event queue for protocol messages
///
/// Event dispatching in wayland is made on a queue basis, allowing you
/// to organise your objects into different queues that can be dispatched
/// independently, for example from different threads.
///
/// And `EventQueue` is not `Send`, and thus must stay on the thread on which
/// they were created. However the `Display` object is `Send + Sync`, allowing
/// you to create the queues directly in the threads that host them.
///
/// When a queue is dispatched (via the `dispatch()` or `dispatch_pending()` methods)
/// all the incoming messages from the server destinated to objects associated with
/// the queue are processed sequentially, and the appropriate implementation for each
/// is invoked. When all messages have been processed these methods return.
///
/// Thus, a typical single-queue event loop for a simple wayland app can be:
///
/// ```no_run
/// # extern crate wayland_client;
/// # use wayland_client::{Display};
/// # fn main() {
/// #     let (display, mut event_queue) = Display::connect_to_env().unwrap();
/// loop {
///     display.flush().unwrap();
///     event_queue.dispatch().expect("An error occured during event dispatching!");
/// }
/// # }
/// ```
pub struct EventQueue {
    // EventQueue is *not* Send
    inner: Rc<EventQueueInner>,
}

/// A token representing this event queue
///
/// This token can be cloned and is meant to allow easier
/// interaction with other functions in the library that
/// require the specification of an event queue, like
/// `Proxy::make_wrapper` and `NewProxy::implement_nonsend`.
pub struct QueueToken {
    inner: Rc<EventQueueInner>,
}

impl EventQueue {
    #[cfg(feature = "native_lib")]
    pub(crate) unsafe fn new(inner: Arc<DisplayInner>, evq: Option<*mut wl_event_queue>) -> EventQueue {
        EventQueue {
            inner: Rc::new(EventQueueInner {
                inner: inner,
                wlevq: evq,
            }),
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
            let ret = match self.inner.wlevq {
                Some(evq) => unsafe {
                    ffi_dispatch!(
                        WAYLAND_CLIENT_HANDLE,
                        wl_display_dispatch_queue,
                        self.inner.inner.ptr(),
                        evq
                    )
                },
                None => unsafe {
                    ffi_dispatch!(
                        WAYLAND_CLIENT_HANDLE,
                        wl_display_dispatch,
                        self.inner.inner.ptr()
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
            let ret = match self.inner.wlevq {
                Some(evq) => unsafe {
                    ffi_dispatch!(
                        WAYLAND_CLIENT_HANDLE,
                        wl_display_dispatch_queue_pending,
                        self.inner.inner.ptr(),
                        evq
                    )
                },
                None => unsafe {
                    ffi_dispatch!(
                        WAYLAND_CLIENT_HANDLE,
                        wl_display_dispatch_pending,
                        self.inner.inner.ptr()
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
                match self.inner.wlevq {
                    Some(evtq) => ffi_dispatch!(
                        WAYLAND_CLIENT_HANDLE,
                        wl_display_roundtrip_queue,
                        self.inner.inner.ptr(),
                        evtq
                    ),
                    None => ffi_dispatch!(
                        WAYLAND_CLIENT_HANDLE,
                        wl_display_roundtrip,
                        self.inner.inner.ptr()
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

    /// Create a new token associated with this event queue
    ///
    /// See `QueueToken` documentation for its use.
    pub fn get_token(&self) -> QueueToken {
        QueueToken {
            inner: self.inner.clone(),
        }
    }
}

impl QueueToken {
    pub(crate) unsafe fn assign_proxy(&self, proxy: *mut wl_proxy) {
        #[cfg(not(feature = "native_lib"))]
        {}
        #[cfg(feature = "native_lib")]
        {
            ffi_dispatch!(
                WAYLAND_CLIENT_HANDLE,
                wl_proxy_set_queue,
                proxy,
                self.inner.wlevq.unwrap_or(ptr::null_mut())
            )
        }
    }
}

impl Drop for EventQueueInner {
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
