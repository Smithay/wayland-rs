//! Utilities for using an [`EventQueue`] from wayland-client with an event loop that performs polling with
//! [`calloop`](https://crates.io/crates/calloop).

use std::{
    io,
    os::unix::io::{AsRawFd, RawFd},
};

use crate::{log_error, DispatchError, EventQueue};
use calloop::{
    generic::Generic, EventSource, InsertError, Interest, LoopHandle, Mode, Poll, PostAction,
    Readiness, RegistrationToken, Token, TokenFactory,
};
use nix::errno::Errno;
use wayland_backend::client::{ReadEventsGuard, WaylandError};

/// An adapter to insert an [`EventQueue`] into a calloop [`EventLoop`](calloop::EventLoop).
///
/// This type implements [`EventSource`] which generates an event whenever events on the event queue need to be
/// dispatched. The event queue available in the callback calloop registers may be used to dispatch pending
/// events using [`EventQueue::dispatch_pending`].
///
/// [`WaylandSource::insert`] can be used to insert this source into an event loop and automatically dispatch
/// pending events on the event queue.
#[derive(Debug)]
pub struct WaylandSource<D> {
    queue: EventQueue<D>,
    fd: Generic<RawFd>,
    read_guard: Option<ReadEventsGuard>,
}

impl<D> WaylandSource<D> {
    /// Wrap an [`EventQueue`] as a [`WaylandSource`].
    pub fn new(queue: EventQueue<D>) -> Result<WaylandSource<D>, WaylandError> {
        let guard = queue.prepare_read()?;
        let fd = Generic::new(guard.connection_fd().as_raw_fd(), Interest::READ, Mode::Level);
        drop(guard);

        Ok(WaylandSource { queue, fd, read_guard: None })
    }

    /// Access the underlying event queue
    ///
    /// Note that you should be careful when interacting with it if you invoke methods that
    /// interact with the wayland socket (such as `dispatch()` or `prepare_read()`). These may
    /// interfere with the proper waking up of this event source in the event loop.
    pub fn queue(&mut self) -> &mut EventQueue<D> {
        &mut self.queue
    }

    /// Insert this source into the given event loop.
    ///
    /// This adapter will pass the event loop's shared data as the `D` type for the event loop.
    pub fn insert(self, handle: LoopHandle<D>) -> Result<RegistrationToken, InsertError<Self>>
    where
        D: 'static,
    {
        handle.insert_source(self, |_, queue, data| queue.dispatch_pending(data))
    }
}

impl<D> EventSource for WaylandSource<D> {
    type Event = ();

    /// The underlying event queue.
    ///
    /// You should call [`EventQueue::dispatch_pending`] inside your callback using this queue.
    type Metadata = EventQueue<D>;
    type Ret = Result<usize, DispatchError>;
    type Error = calloop::Error;

    fn process_events<F>(
        &mut self,
        readiness: Readiness,
        token: Token,
        mut callback: F,
    ) -> Result<PostAction, Self::Error>
    where
        F: FnMut(Self::Event, &mut Self::Metadata) -> Self::Ret,
    {
        let queue = &mut self.queue;
        let read_guard = &mut self.read_guard;

        let action = self.fd.process_events(readiness, token, |_, _| {
            // 1. read events from the socket if any are available
            if let Some(guard) = read_guard.take() {
                // might be None if some other thread read events before us, concurrently
                if let Err(WaylandError::Io(err)) = guard.read() {
                    if err.kind() != io::ErrorKind::WouldBlock {
                        return Err(err);
                    }
                }
            }

            // 2. dispatch any pending events in the queue
            // This is done to ensure we are not waiting for messages that are already in the buffer.
            Self::loop_callback_pending(queue, &mut callback)?;
            *read_guard = Some(Self::prepare_read(queue)?);

            // 3. Once dispatching is finished, flush the responses to the compositor
            if let Err(WaylandError::Io(e)) = queue.flush() {
                if e.kind() != io::ErrorKind::WouldBlock {
                    // in case of error, forward it and fast-exit
                    return Err(e);
                }
                // WouldBlock error means the compositor could not process all our messages
                // quickly. Either it is slowed down or we are a spammer.
                // Should not really happen, if it does we do nothing and will flush again later
            }

            Ok(PostAction::Continue)
        })?;

        Ok(action)
    }

    fn register(
        &mut self,
        poll: &mut Poll,
        token_factory: &mut TokenFactory,
    ) -> calloop::Result<()> {
        self.fd.register(poll, token_factory)
    }

    fn reregister(
        &mut self,
        poll: &mut Poll,
        token_factory: &mut TokenFactory,
    ) -> calloop::Result<()> {
        self.fd.reregister(poll, token_factory)
    }

    fn unregister(&mut self, poll: &mut Poll) -> calloop::Result<()> {
        self.fd.unregister(poll)
    }

    fn pre_run<F>(&mut self, mut callback: F) -> calloop::Result<()>
    where
        F: FnMut((), &mut Self::Metadata) -> Self::Ret,
    {
        debug_assert!(self.read_guard.is_none());

        // flush the display before starting to poll
        if let Err(WaylandError::Io(err)) = self.queue.flush() {
            if err.kind() != io::ErrorKind::WouldBlock {
                // in case of error, don't prepare a read, if the error is persistent, it'll trigger in other
                // wayland methods anyway
                log_error!("Error trying to flush the wayland display: {}", err);
                return Err(err.into());
            }
        }

        // ensure we are not waiting for messages that are already in the buffer.
        Self::loop_callback_pending(&mut self.queue, &mut callback)?;
        self.read_guard = Some(Self::prepare_read(&mut self.queue)?);

        Ok(())
    }

    fn post_run<F>(&mut self, _: F) -> calloop::Result<()>
    where
        F: FnMut((), &mut Self::Metadata) -> Self::Ret,
    {
        // Drop implementation of ReadEventsGuard will do cleanup
        self.read_guard.take();
        Ok(())
    }
}

impl<D> WaylandSource<D> {
    /// Loop over the callback until all pending messages have been dispatched.
    fn loop_callback_pending<F>(queue: &mut EventQueue<D>, callback: &mut F) -> io::Result<()>
    where
        F: FnMut((), &mut EventQueue<D>) -> Result<usize, DispatchError>,
    {
        // Loop on the callback until no pending events are left.
        loop {
            match callback((), queue) {
                // No more pending events.
                Ok(0) => break Ok(()),

                Ok(_) => continue,

                Err(DispatchError::Backend(WaylandError::Io(err))) => {
                    return Err(err);
                }

                Err(DispatchError::Backend(WaylandError::Protocol(err))) => {
                    log_error!("Protocol error received on display: {}", err);

                    break Err(Errno::EPROTO.into());
                }

                Err(DispatchError::BadMessage { interface, sender_id, opcode }) => {
                    log_error!(
                        "Bad message on interface \"{}\": (sender_id: {}, opcode: {})",
                        interface,
                        sender_id,
                        opcode,
                    );

                    break Err(Errno::EPROTO.into());
                }
            }
        }
    }

    fn prepare_read(queue: &mut EventQueue<D>) -> io::Result<ReadEventsGuard> {
        queue.prepare_read().map_err(|err| match err {
            WaylandError::Io(err) => err,

            WaylandError::Protocol(err) => {
                log_error!("Protocol error received on display: {}", err);
                Errno::EPROTO.into()
            }
        })
    }
}
