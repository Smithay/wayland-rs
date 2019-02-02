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
/// There are two main ways to driving an event queue forward. The first way is the
/// simplest and generally sufficient for single-threaded apps that only process events
/// from wayland. It consists of using the `EventQueue::dispatch()` method, which will
/// take care of sending pending requests to the server, block until some events are
/// available, read them, and call the associated handlers:
///
/// ```no_run
/// # extern crate wayland_client;
/// # use wayland_client::{Display};
/// # fn main() {
/// #     let (display, mut event_queue) = Display::connect_to_env().unwrap();
/// loop {
///     // The dispatch() method returns once it has received some events to dispatch
///     // and have emptied the wayland socket from its pending messages, so it needs
///     // to be called in a loop. If this method returns an error, your connection to
///     // the wayland server is very likely dead. See its documentation for more details.
///     event_queue.dispatch().expect("An error occurred during event dispatching!");
/// }
/// # }
/// ```
///
/// The second way is more appropriate for apps that are either multithreaded (and need to process
/// wayland events from different threads conccurently) or need to react to events from different
/// sources and can't affort to just block on the wayland socket. It centers around three methods:
/// `Display::flush()`, `EventQueue::read_events()` and `EventQueue::dispatch_pending()`:
///
/// ```no_run
/// # extern crate wayland_client;
/// # use wayland_client::Display;
/// # fn main() {
/// # let (display, mut event_queue) = Display::connect_to_env().unwrap();
/// loop {
///     // The first method, called on the Display, is flush(). It writes all pending
///     // requests to the socket. Calling it ensures that the server will indeed
///     // receive your requests (so it can react to them).
///     if let Err(e) = display.flush() {
///         if e.kind() != ::std::io::ErrorKind::WouldBlock {
///             // if you are sending a realy large number of request, it might fill
///             // the internal buffers of the socket, in which case you should just
///             // retry flushing later. Other errors are a problem though.
///             eprintln!("Error while trying to flush the wayland socket: {:?}", e);
///         }
///     }
///
///     // The second method will try to read events from the socket. It is done in two
///     // steps, first the read is prepared, and then it is actually executed. This allows
///     // lower contention when different threads are trying to trigger a read of events
///     // concurently
///     if let Some(guard) = event_queue.prepare_read() {
///         // prepare_read() returns None if there are already events pending in this
///         // event queue, in which case there is no need to try to read from the socket
///         if let Err(e) = guard.read_events() {
///             if e.kind() != ::std::io::ErrorKind::WouldBlock {
///                 // if read_events() returns Err(WouldBlock), this just means that no new
///                 // messages are available to be read
///                 eprintln!("Error while trying to read from the wayland socket: {:?}", e);
///             }
///         }
///     }
///
///     // Then, once events have been read from the socket and stored in the internal
///     // queues, they need to be dispatched to their handler. Note that while flush()
///     // and read_events() are global and will affect the whole connection, this last
///     // method will only affect the event queue it is being called on. This method
///     // cannot error unless there is a bug in the server or a previous read of events
///     // already errored.
///     event_queue.dispatch_pending().expect("Failed to dispatch all messages.");
///
///     // Note that none of these methods are blocking, as such they should not be used
///     // as a loop as-is if there are no other sources of events your program is waiting on.
///
///     // The wayland socket can also be integrated in a poll-like mechanism, using
///     // either the integration with calloop provided by the "eventloop" cargo feature,
///     // or the get_connection_fd() method.
/// }
/// # }
/// ```
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
    /// If an error is returned, your connection with the wayland compositor is probably lost.
    /// You may want to check `Display::protocol_error()` to see if it was caused by a protocol error.
    pub fn dispatch(&mut self) -> io::Result<u32> {
        self.inner.dispatch()
    }

    /// Dispatches pending events from the internal buffer.
    ///
    /// Dispatches all events to their appropriators.
    /// Never blocks, if no events were pending, simply returns
    /// `Ok(0)`.
    ///
    /// If an error is returned, your connection with the wayland compositor is probably lost.
    /// You may want to check `Display::protocol_error()` to see if it was caused by a protocol error.
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
    /// If an error is returned, your connection with the wayland compositor is probably lost.
    /// You may want to check `Display::protocol_error()` to see if it was caused by a protocol error.
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

    /// Retrieve the file descriptor associated with the wayland socket
    ///
    /// This FD should only be used to integrate into a polling mechanism, and should
    /// never be directly read from or written to.
    pub fn get_connection_fd(&self) -> ::std::os::unix::io::RawFd {
        self.inner.get_connection_fd()
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
