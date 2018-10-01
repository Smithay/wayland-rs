use std::io;
use std::rc::Rc;

use imp::EventQueueInner;

/// An event queue for protocol messages
///
/// Event dispatching in wayland is made on a queue basis, allowing you
/// to organize your objects into different queues that can be dispatched
/// independently, for example from different threads.
///
/// And `EventQueue` is not `Send`, and thus must stay on the thread on which
/// they were created. However the `Display` object is `Send + Sync`, allowing
/// you to create the queues directly in the threads that host them.
///
/// When a queue is dispatched (via the `dispatch()` or `dispatch_pending()` methods)
/// all the incoming messages from the server designated to objects associated with
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
///     event_queue.dispatch().expect("An error occurred during event dispatching!");
/// }
/// # }
/// ```
///
/// See `EventQueue::prepare_read()` if you need more control about when the connection
/// socket is read. This will typically the case if you need to integrate other sources
/// of event into the event loop of your application.
pub struct EventQueue {
    // EventQueue is *not* Send
    pub(crate) inner: Rc<EventQueueInner>,
}

/// A token representing this event queue
///
/// This token can be cloned and is meant to allow easier
/// interaction with other functions in the library that
/// require the specification of an event queue, like
/// `Proxy::make_wrapper` and `NewProxy::implement_nonsend`.
pub struct QueueToken {
    pub(crate) inner: Rc<EventQueueInner>,
}

impl EventQueue {
    pub(crate) fn new(inner: EventQueueInner) -> EventQueue {
        EventQueue {
            inner: Rc::new(inner),
        }
    }
    /// Dispatches events from the internal buffer.
    ///
    /// Dispatches all events to their appropriators.
    /// If no events were in the internal buffer, will block until
    /// some events are read and dispatch them.
    /// This process can insert events in the internal buffers of
    /// other event queues.
    ///
    /// If an error is returned, your connection with the wayland
    /// compositor is probably lost.
    pub fn dispatch(&mut self) -> io::Result<u32> {
        self.inner.dispatch()
    }

    /// Dispatches pending events from the internal buffer.
    ///
    /// Dispatches all events to their appropriators.
    /// Never blocks, if no events were pending, simply returns
    /// `Ok(0)`.
    ///
    /// If an error is returned, your connection with the wayland
    /// compositor is probably lost.
    pub fn dispatch_pending(&mut self) -> io::Result<u32> {
        self.inner.dispatch_pending()
    }

    /// Synchronous roundtrip
    ///
    /// This call will cause a synchronous roundtrip with the wayland server. It will block until all
    /// pending requests of this queue are sent to the server and it has processed all of them and
    /// send the appropriate events.
    ///
    /// Handlers are called as a consequence.
    ///
    /// On success returns the number of dispatched events.
    pub fn sync_roundtrip(&mut self) -> io::Result<u32> {
        self.inner.sync_roundtrip()
    }

    /// Create a new token associated with this event queue
    ///
    /// See `QueueToken` documentation for its use.
    pub fn get_token(&self) -> QueueToken {
        QueueToken {
            inner: self.inner.clone(),
        }
    }

    /// Prepare an concurrent read
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
        match self.inner.prepare_read() {
            Ok(()) => Some(ReadEventsGuard {
                inner: self.inner.clone(),
                done: false,
            }),
            Err(()) => None,
        }
    }
}

/// A guard over a read intention.
///
/// See `EventQueue::prepare_read()` for details about its use.
pub struct ReadEventsGuard {
    inner: Rc<EventQueueInner>,
    done: bool,
}

impl ReadEventsGuard {
    /// Read events
    ///
    /// Reads events from the server socket. If other `ReadEventsGuard` exists, will block
    /// until they are all consumed or destroyed.
    pub fn read_events(mut self) -> io::Result<i32> {
        self.done = true;
        self.inner.read_events()
    }

    /// Cancel the read
    ///
    /// Will cancel the read intention associated with this guard. Never blocks.
    ///
    /// Has the same effect as letting the guard go out of scope.
    pub fn cancel(mut self) {
        // just run the destructor
        self.done = true;
    }
}

impl Drop for ReadEventsGuard {
    fn drop(&mut self) {
        if !self.done {
            self.inner.cancel_read();
        }
    }
}
