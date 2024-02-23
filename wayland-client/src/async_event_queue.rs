use crate::{
    conn::SyncData, protocol::wl_display, Connection, DispatchError, EventQueue, WaylandError,
};
use std::{
    io,
    num::NonZeroUsize,
    os::fd::{AsFd, BorrowedFd},
    sync::{atomic::Ordering, Arc},
};
use tokio::io::{unix::AsyncFd, Ready};

/// Asynchronous version of [`EventQueue`], taking borrowed references to an [`EventQueue`],
/// allowing for non-blocking polled asynchronous operations against the Queue using
/// [`AsyncFd`] from the tokio runtime to poll against the wayland socket
#[derive(Debug)]
pub struct AsyncEventQueue<'a, State> {
    /// Inner reference to synchronous version of this event queue
    pub queue: EventQueue<State>,
    conn: &'a Connection,
    afd: AsyncFd<BorrowedFd<'a>>,
}

impl<'a, State> AsyncEventQueue<'a, State> {
    /// Creates a new [`AsyncEventQueue`] from a reference to a given [`Connection`] and
    /// *synchronous* [`EventQueue`]
    #[inline]
    pub fn new(conn: &'a Connection, event_queue: EventQueue<State>) -> io::Result<Self> {
        Ok(AsyncEventQueue { queue: event_queue, conn, afd: AsyncFd::new(conn.as_fd())? })
    }

    #[inline(always)]
    fn dispatch_pending(
        &mut self,
        data: &mut State,
    ) -> Result<Option<NonZeroUsize>, DispatchError> {
        self.queue.dispatch_pending(data).map(NonZeroUsize::new)
    }

    /// Asynchronous version of [`EventQueue::blocking_dispatch`] that uses tokio's fd-based
    /// polling system to avoid blocking.
    ///
    /// This function completes once any number of events have been dispatched *on this thread*.
    ///
    /// In the success case, returns the # of events that were dispatched.
    pub async fn dispatch_single(&mut self, data: &mut State) -> Result<usize, DispatchError> {
        self.queue.flush_d()?;
        if let Some(v) = self.dispatch_pending(data)? {
            return Ok(v.get());
        }
        loop {
            let mut guard = self
                .afd
                .readable_mut()
                .await
                .map_err(|e| DispatchError::Backend(WaylandError::from(e)))?;
            let lock = if let Some(v) = self.queue.prepare_read() {
                v
            } else if let Some(v) = self.dispatch_pending(data)? {
                return Ok(v.get());
            } else {
                continue;
            };
            let ret = match lock.read() {
                Ok(..) => Some(self.queue.dispatch_pending(data)),
                Err(ref e) if e.would_block() => {
                    guard.clear_ready_matching(Ready::READABLE);
                    None
                }
                Err(e) => Some(Err(DispatchError::from(e))),
            };
            if let Some(ret) = ret {
                return ret;
            }
        }
    }

    /// Asynchronous version of [`EventQueue::roundtrip`]
    ///
    /// Instead of blocking, this [`Future`](std::future::Future) will complete once the roundtrip
    /// has completed
    ///
    /// This function may be useful during initial setup of your app. This function may also be useful
    /// where you need to guarantee all requests prior to calling this function are completed.
    pub async fn roundtrip(&mut self, data: &mut State) -> Result<usize, DispatchError> {
        let done = Arc::new(SyncData::default());
        let display = self.conn.display();
        self.conn
            .send_request(&display, wl_display::Request::Sync {}, Some(done.clone()))
            .map_err(|_| WaylandError::Io(rustix::io::Errno::PIPE.into()))?;
        let mut n = 0usize;
        while !done.done.load(Ordering::Relaxed) {
            n += self.dispatch_single(data).await?;
        }
        Ok(n)
    }

    /// Converts this AsyncEventQueue back into it's synchronous variant
    #[inline(always)]
    pub fn unwrap(self) -> EventQueue<State> {
        self.into()
    }
}

// impl<'a, State> Into<EventQueue<State>> for AsyncEventQueue<'a, State> {
//     #[inline(always)]
//     fn into(self) -> EventQueue<State> {
//         self.queue
//     }
// }

trait EventQueueExt<State> {
    fn flush_d(&self) -> Result<(), DispatchError>;
}

impl<State> EventQueueExt<State> for EventQueue<State> {
    #[inline(always)]
    fn flush_d(&self) -> Result<(), DispatchError> {
        self.flush().map_err(DispatchError::Backend)
    }
}

/// Represents types of errors that might indicate that the error was that the operation would have
/// blocked
trait MaybeBlockingError {
    /// Returns `true` if self indicates an error meaning that an operation would have blocked
    fn would_block(&self) -> bool;
}

impl MaybeBlockingError for io::Error {
    #[inline(always)]
    fn would_block(&self) -> bool {
        use io::ErrorKind;
        self.kind() == ErrorKind::WouldBlock
    }
}

impl MaybeBlockingError for WaylandError {
    #[inline]
    fn would_block(&self) -> bool {
        match self {
            WaylandError::Io(e) => e.would_block(),
            _ => false,
        }
    }
}

/// Additional methods available on [`Connection`] when an async runtime is available
pub trait ConnectionExtAsync {
    /// Creates a new *asynchronous* event queue, containing the downstream [`EventQueue`]
    fn new_async_event_queue<State>(&self) -> io::Result<AsyncEventQueue<State>>;
}

impl ConnectionExtAsync for Connection {
    #[inline]
    fn new_async_event_queue<State>(&self) -> io::Result<AsyncEventQueue<State>> {
        AsyncEventQueue::new(self, self.new_event_queue())
    }
}
