use std::io::{Error as IoError, Result as IoResult};
use std::ptr;
use std::rc::Rc;
use std::sync::Arc;

use display::DisplayInner;

#[cfg(feature = "native_lib")]
use wayland_sys::client::*;

struct EventQueueInner {
    #[cfg(feature = "native_lib")]
    wlevq: Option<*mut wl_event_queue>,
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
///
/// See `EventQueue::prepare_read()` if you need more control about when the connection
/// socket is read. This will typically the case if you need to integrate other sources
/// of event into the event loop of your application.
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
    /// If no events were in the internal buffer, will block until
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
                    ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_dispatch, self.inner.inner.ptr())
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
    /// Never blocks, if no events were pending, simply returns
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

    /// Prepare an conccurent read
    ///
    /// Will declare your intention to read events from the server socket.
    ///
    /// Will return `None` if there are still some events awaiting dispatch on this EventIterator.
    /// In this case, you need to call `dispatch_pending()` before calling this method again.
    ///
    /// As long as the returned guard is in scope, no events can be dispatched to any event iterator.
    ///
    /// The guard can then be destroyed by two means:
    ///
    ///  - Calling its `cancel()` method (or letting it go out of scope): the read intention will
    ///    be cancelled
    ///  - Calling its `read_events()` method: will block until all existing guards are destroyed
    ///    by one of these methods, then events will be read and all blocked `read_events()` calls
    ///    will return.
    ///
    /// This call will otherwise not block on the server socket if it is empty, and return
    /// an io error `WouldBlock` in such cases.
    pub fn prepare_read(&self) -> Option<ReadEventsGuard> {
        let ret = unsafe {
            match self.inner.wlevq {
                Some(evtq) => ffi_dispatch!(
                    WAYLAND_CLIENT_HANDLE,
                    wl_display_prepare_read_queue,
                    self.inner.inner.ptr(),
                    evtq
                ),
                None => ffi_dispatch!(
                    WAYLAND_CLIENT_HANDLE,
                    wl_display_prepare_read,
                    self.inner.inner.ptr()
                ),
            }
        };
        if ret >= 0 {
            Some(ReadEventsGuard {
                inner: self.inner.clone(),
            })
        } else {
            None
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

/// A guard over a read intention.
///
/// See `EventQueue::prepare_read()` for details about its use.
pub struct ReadEventsGuard {
    inner: Rc<EventQueueInner>,
}

impl ReadEventsGuard {
    /// Read events
    ///
    /// Reads events from the server socket. If other `ReadEventsGuard` exists, will block
    /// until they are all consumed or destroyed.
    pub fn read_events(self) -> IoResult<i32> {
        let ret = unsafe {
            ffi_dispatch!(
                WAYLAND_CLIENT_HANDLE,
                wl_display_read_events,
                self.inner.inner.ptr()
            )
        };
        // Don't run destructor that would cancel the read intent
        ::std::mem::forget(self);
        if ret >= 0 {
            Ok(ret)
        } else {
            Err(IoError::last_os_error())
        }
    }

    /// Cancel the read
    ///
    /// Will cancel the read intention associated with this guard. Never blocks.
    ///
    /// Has the same effet as letting the guard go out of scope.
    pub fn cancel(self) {
        // just run the destructor
    }
}

impl Drop for ReadEventsGuard {
    fn drop(&mut self) {
        unsafe {
            ffi_dispatch!(
                WAYLAND_CLIENT_HANDLE,
                wl_display_cancel_read,
                self.inner.inner.ptr()
            )
        }
    }
}
