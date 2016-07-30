use std::cell::{Cell, UnsafeCell};
use std::collections::VecDeque;
use std::io;
use std::sync::Arc;

use wayland_sys::client::*;

/// All possible wayland events.
///
/// This enum does a first sorting of event, with a variant for each
/// protocol extension activated in cargo features.
///
/// You don't need to match against events from protocol extensions you don't use,
/// as such events cannot be generated if you don't bind the globals marking the entry
/// points of these protocols.
///
/// As the number of variant of this enum can change depending on which cargo features are
/// activated, it should *never* be matched exhaustively. It contains an hidden, never-used
/// variant to ensure it.
#[derive(Debug)]
pub enum Event {
    /// An event from the core wayland protocol
    Wayland(::wayland::WaylandProtocolEvent),
    #[cfg(feature = "wp-presentation_time")]
    /// An event from the protocol extension presentation-time
    PresentationTime(::extensions::presentation_time::PresentationTimeProtocolEvent),
    #[cfg(feature = "wp-viewporter")]
    /// An event from the protocol extension viewporter
    Viewporter(::extensions::viewporter::ViewporterProtocolEvent),
    #[cfg(all(feature = "unstable-protocols", feature="wpu-xdg_shell"))]
    /// An event from the protocol extension xdg-shell
    XdgShellUnstableV5(::extensions::xdg_shell::XdgShellUnstableV5ProtocolEvent),
    /// An event from the protocol extension desktop-shell
    Desktop(::extensions::desktop_shell::DesktopProtocolEvent),
    #[doc(hidden)]
    __DoNotMatchThis,
}

pub struct EventFifo {
    queue: UnsafeCell<VecDeque<Event>>,
    alive: Cell<bool>
}

unsafe impl Send for EventFifo {}
unsafe impl Sync for EventFifo {}

impl EventFifo {
    pub fn new() -> EventFifo {
        EventFifo {
            queue: UnsafeCell::new(VecDeque::new()),
            alive: Cell::new(true)
        }
    }

    pub unsafe fn push(&self, evt: Event) {
        (&mut *self.queue.get()).push_front(evt)
    }

    unsafe fn pop(&self) -> Option<Event> {
        (&mut *self.queue.get()).pop_back()
    }

    pub unsafe fn alive(&self) -> bool {
        self.alive.get()
    }
}

/// An event iterator
///
/// Each one is linked to a wayand event queue, and will collect
/// events from the wayand objects attached to it.
///
/// Its primary interface is through the `Iterator` trait. Note that
/// unlike most traditionnal iterators, it can start yielding again events
/// after returning `None`. It is thus not recommended to use constructs
/// that consume the iterator.
///
/// If any error is encountered, the `next()` method from `Iterator` will panic,
/// as all these errors are fatal to the wayland connection. If you need to handle
/// them gracefully, use the `next_event_dispatch()` or `next_event()` methods instead.
///
/// A typical event loop using surch an iterator would look like this:
///
/// ```no_run
/// # let (_, mut event_iterator) = wayland_client::get_display().unwrap();
/// loop {
///     for event in &mut event_iterator {
///         /* handle the event */
///     }
///     event_iterator.dispatch().expect("Connection with the compositor was lost.");
/// }
/// ```
pub struct EventIterator {
    fifo: Arc<EventFifo>,
    event_queue: Option<*mut wl_event_queue>,
    display: *mut wl_display
}

impl EventIterator {
    /// Retrieves the next event in this iterator.
    ///
    /// Will automatically try to dispatch pending events if necessary.
    ///
    /// Similar to a combination of `next_event` and `dispatch_pending`.
    pub fn next_event_dispatch(&mut self) -> io::Result<Option<Event>> {
        if let Some(evt) = unsafe { self.fifo.pop() } {
            return Ok(Some(evt))
        } else {
            self.dispatch_pending().map(|_| unsafe { self.fifo.pop() })
        }
    }

    /// Retrieves the next event in this iterator.
    ///
    /// Returns `None` if no event is available. Some events might still be in the
    /// internal buffer, waiting to be dispatched to their EventIterators. Use
    /// `dispatch_pending()` to dispatch the waiting events to this iterator.
    pub fn next_event(&mut self) -> Option<Event> {
        unsafe { self.fifo.pop() }
    }

    /// Synchronous roundtrip
    ///
    /// This call will cause a synchonous roundtrip with the wayland server. It will block until all
    /// pending requests of this queue are send to the server and it has processed all of them and
    /// send the appropriate events.
    ///
    /// On success returns the number of dispatched events.
    pub fn sync_roundtrip(&mut self) -> io::Result<i32> {
        let ret = unsafe { match self.event_queue {
            Some(evtq) => {
                ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_roundtrip_queue,
                    self.display, evtq)
            }
            None => {
                ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_roundtrip, self.display)
            }
        }};
        if ret >= 0 { Ok(ret) } else { Err(io::Error::last_os_error()) }
    }


    /// Non-blocking dispatch
    ///
    /// Will dispatch all pending events from the internal buffer to this event iterator.
    /// Will not try to read events from the server socket, hence never blocks.
    ///
    /// On success returns the number of dispatched events.
    pub fn dispatch_pending(&mut self) -> io::Result<i32> {
        let ret = unsafe { match self.event_queue {
            Some(evtq) => {
                ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_dispatch_queue_pending,
                    self.display, evtq)
            },
            None => {
                ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_dispatch_pending,
                    self.display)
            }
        }};
        if ret >= 0 { Ok(ret) } else { Err(io::Error::last_os_error()) }
    }

    /// Blocking dispatch
    ///
    /// Will dispatch all pending events from the internal buffer to this event iterator.
    /// If the buffer was empty, will read new events from the server socket, blocking if necessary.
    ///
    /// On success returns the number of dispatched events.
    ///
    /// Can cause a deadlock if called several times conccurently on different `EventIterator`.
    /// For a risk-free approach, use `prepare_read()` and `dispatch_pending()`
    pub fn dispatch(&mut self) -> io::Result<i32> {
        let ret = unsafe { match self.event_queue {
            Some(evtq) => {
                ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_dispatch_queue,
                    self.display, evtq)
            },
            None => {
                ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_dispatch,
                    self.display)
            }
        }};
        if ret >= 0 { Ok(ret) } else { Err(io::Error::last_os_error()) }
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
        let ret = unsafe { match self.event_queue {
            Some(evtq) => {
                ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_prepare_read_queue,
                    self.display, evtq)
            },
            None => {
                ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_prepare_read,
                    self.display)
            }
        }};
        if ret >= 0 { Some(ReadEventsGuard { display: self.display }) } else { None }
    }
}

impl Iterator for EventIterator {
    type Item = Event;
    fn next(&mut self) -> Option<Event> {
        match self.next_event_dispatch() {
            Ok(evt) => evt,
            Err(e) => {
                panic!("Connexion with wayland compositor was lost: {:?}", e)
            }
        }
    }
}

impl Drop for EventIterator {
    fn drop(&mut self) {
        self.fifo.alive.set(false);
        if let Some(evq) = self.event_queue {
            unsafe {
                ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_event_queue_destroy, evq)
            }
        }
    }
}

pub fn create_event_iterator(display: *mut wl_display, event_queue: Option<*mut wl_event_queue>) -> EventIterator {
    EventIterator {
        fifo: Arc::new(EventFifo::new()),
        event_queue: event_queue,
        display: display
    }
}

pub fn get_eventiter_internals(evt: &EventIterator) -> Arc<EventFifo> {
    evt.fifo.clone()
}

/// A guard over a read intention.
///
/// See `WlDisplay::prepare_read()` for details about its use.
pub struct ReadEventsGuard {
    display: *mut wl_display
}

impl ReadEventsGuard {
    /// Read events
    ///
    /// Reads events from the server socket. If other `ReadEventsGuard` exists, will block
    /// until they are all destroyed.
    pub fn read_events(self) -> io::Result<i32> {
        let ret = unsafe { ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_read_events, self.display) };
        // Don't run destructor that would cancel the read intent
        ::std::mem::forget(self);
        if ret >= 0 { Ok(ret) } else { Err(io::Error::last_os_error()) }
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
        unsafe { ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_cancel_read, self.display) }
    }
}
