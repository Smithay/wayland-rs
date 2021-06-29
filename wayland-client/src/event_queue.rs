use std::{io, rc::Rc};

use crate::imp::EventQueueInner;
use crate::{AnonymousObject, DispatchData, Display, Main, RawEvent};

/// An event queue for protocol messages
///
/// Event dispatching in wayland is made on a queue basis, allowing you
/// to organize your objects into different queues that can be dispatched
/// independently, for example from different threads.
///
/// An `EventQueue` is not `Send`, and thus must stay on the thread on which
/// it was created. However the `Display` object is `Send + Sync`, allowing
/// you to create the queues directly on the threads that host them.
///
/// When a queue is dispatched (via the `dispatch(..)` or `dispatch_pending(..)` methods)
/// all the incoming messages from the server designated to objects associated with
/// the queue are processed sequentially, and the appropriate implementation for each
/// is invoked. When all messages have been processed these methods return.
///
/// There are two main ways to driving an event queue forward. The first way is the
/// simplest and generally sufficient for single-threaded apps that only process events
/// from wayland. It consists of using the `EventQueue::dispatch(..)` method, which will
/// take care of sending pending requests to the server, block until some events are
/// available, read them, and call the associated handlers:
///
/// ```no_run
/// # extern crate wayland_client;
/// # use wayland_client::{Display};
/// # let display = Display::connect_to_env().unwrap();
/// # let mut event_queue = display.create_event_queue();
/// loop {
///     // The dispatch() method returns once it has received some events to dispatch
///     // and have emptied the wayland socket from its pending messages, so it needs
///     // to be called in a loop. If this method returns an error, your connection to
///     // the wayland server is very likely dead. See its documentation for more details.
///     event_queue.dispatch(&mut (), |_,_,_| {
///         /* This closure will be called for every event received by an object not
///            assigned to any Filter. If you plan to assign all your objects to Filter,
///            the simplest thing to do is to assert this is never called. */
///         unreachable!();
///     }).expect("An error occurred during event dispatching!");
/// }
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
/// # let display = Display::connect_to_env().unwrap();
/// # let mut event_queue = display.create_event_queue();
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
///     event_queue.dispatch_pending(&mut (), |_,_,_| {}).expect("Failed to dispatch all messages.");
///
///     // Note that none of these methods are blocking, as such they should not be used
///     // as a loop as-is if there are no other sources of events your program is waiting on.
///
///     // The wayland socket can also be integrated in a poll-like mechanism by using
///     // the file descriptor provided by the `get_connection_fd()` method.
/// }
/// ```
pub struct EventQueue {
    // EventQueue is *not* Send
    pub(crate) inner: Rc<EventQueueInner>,
    display: Display,
}

impl std::fmt::Debug for EventQueue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("EventQueue { ... }")
    }
}

/// A token representing this event queue
///
/// This token can be cloned and is meant to allow easier
/// interaction with other functions in the library that
/// require the specification of an event queue, like
/// `Proxy::assign`.
#[derive(Clone)]
pub struct QueueToken {
    pub(crate) inner: Rc<EventQueueInner>,
}

impl std::fmt::Debug for QueueToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("QueueToken { ... }")
    }
}

impl EventQueue {
    pub(crate) fn new(inner: EventQueueInner, display: Display) -> EventQueue {
        EventQueue { inner: Rc::new(inner), display }
    }
    /// Dispatches events from the internal buffer.
    ///
    /// Dispatches all events to their appropriate filters.
    /// If no events were in the internal buffer, will block until
    /// some events are read and dispatch them.
    /// This process can insert events in the internal buffers of
    /// other event queues.
    ///
    /// The provided `data` will be mutably accessible from all the callbacks, via the
    /// [`DispatchData`](struct.DispatchData.html) mechanism. If you don't need global data, you
    /// can just provide a `&mut ()` there.
    ///
    /// If an error is returned, your connection with the wayland compositor is probably lost.
    /// You may want to check `Display::protocol_error()` to see if it was caused by a protocol error.
    pub fn dispatch<T: std::any::Any, F>(&mut self, data: &mut T, fallback: F) -> io::Result<u32>
    where
        F: FnMut(RawEvent, Main<AnonymousObject>, DispatchData<'_>),
    {
        let mut data = DispatchData::wrap(data);
        self.inner.dispatch(data.reborrow(), fallback)
    }

    /// Dispatches pending events from the internal buffer.
    ///
    /// Dispatches all events to their appropriate callbacks.
    /// Never blocks, if no events were pending, simply returns
    /// `Ok(0)`.
    ///
    /// The provided `data` will be mutably accessible from all the callbacks, via the
    /// [`DispatchData`](struct.DispatchData.html) mechanism. If you don't need global data, you
    /// can just provide a `&mut ()` there.
    ///
    /// If an error is returned, your connection with the wayland compositor is probably lost.
    /// You may want to check `Display::protocol_error()` to see if it was caused by a protocol error.
    pub fn dispatch_pending<T: std::any::Any, F>(
        &mut self,
        data: &mut T,
        fallback: F,
    ) -> io::Result<u32>
    where
        F: FnMut(RawEvent, Main<AnonymousObject>, DispatchData<'_>),
    {
        let mut data = DispatchData::wrap(data);
        self.inner.dispatch_pending(data.reborrow(), fallback)
    }

    /// Synchronous roundtrip
    ///
    /// This call will cause a synchronous roundtrip with the wayland server. It will block until all
    /// pending requests of this queue are sent to the server and it has processed all of them and
    /// send the appropriate events.
    ///
    /// Handlers are called as a consequence.
    ///
    /// The provided `data` will be mutably accessible from all the callbacks, via the
    /// [`DispatchData`](struct.DispatchData.html) mechanism. If you don't need global data, you
    /// can just provide a `&mut ()` there.
    ///
    /// On success returns the number of dispatched events.
    /// If an error is returned, your connection with the wayland compositor is probably lost.
    /// You may want to check `Display::protocol_error()` to see if it was caused by a protocol error.
    pub fn sync_roundtrip<T: std::any::Any, F>(
        &mut self,
        data: &mut T,
        fallback: F,
    ) -> io::Result<u32>
    where
        F: FnMut(RawEvent, Main<AnonymousObject>, DispatchData<'_>),
    {
        let mut data = DispatchData::wrap(data);
        self.inner.sync_roundtrip(data.reborrow(), fallback)
    }

    /// Create a new token associated with this event queue
    ///
    /// See `QueueToken` documentation for its use.
    pub fn token(&self) -> QueueToken {
        QueueToken { inner: self.inner.clone() }
    }

    /// Prepare an concurrent read
    ///
    /// Will declare your intention to read events from the server socket.
    ///
    /// Will return `None` if there are still some events awaiting dispatch on this EventIterator.
    /// In this case, you need to call `dispatch_pending()` before calling this method again.
    ///
    /// The guard can then be used by two means:
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
            Ok(()) => Some(ReadEventsGuard { inner: self.inner.clone(), done: false }),
            Err(()) => None,
        }
    }

    /// Access the `Display` of the connection
    pub fn display(&self) -> &Display {
        &self.display
    }
}

/// A guard over a read intention.
///
/// See `EventQueue::prepare_read()` for details about its use.
pub struct ReadEventsGuard {
    inner: Rc<EventQueueInner>,
    done: bool,
}

impl std::fmt::Debug for ReadEventsGuard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ReadEventsGuard { ... }")
    }
}

impl ReadEventsGuard {
    /// Read events
    ///
    /// Reads events from the server socket. If other `ReadEventsGuard` exists, will block
    /// until they are all consumed or destroyed.
    pub fn read_events(mut self) -> io::Result<()> {
        self.done = true;
        self.inner.read_events()
    }

    /// Cancel the read
    ///
    /// Will cancel the read intention associated with this guard. Never blocks.
    ///
    /// Has the same effect as letting the guard go out of scope.
    pub fn cancel(self) {
        // just run the destructor
    }
}

impl Drop for ReadEventsGuard {
    fn drop(&mut self) {
        if !self.done {
            self.inner.cancel_read();
        }
    }
}
