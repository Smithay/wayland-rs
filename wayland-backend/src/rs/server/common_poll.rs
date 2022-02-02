use std::{
    os::unix::{
        io::{AsRawFd, RawFd},
        net::UnixStream,
    },
    sync::Arc,
};

use super::ClientData;
use crate::types::server::{DisconnectReason, InitError};

#[cfg(target_os = "linux")]
use nix::sys::epoll::*;

#[cfg(any(
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
use nix::sys::event::*;

use super::{ClientId, Handle};

/// A backend object that represents the state of a wayland server.
///
/// A backend is used to drive a wayland server by receiving requests, dispatching messages to the appropriate
/// handlers and flushes requests to be sent back to the client.
#[derive(Debug)]
pub struct Backend<D> {
    handle: Handle<D>,
    poll_fd: RawFd,
}

impl<D> Backend<D> {
    /// Initialize a new Wayland backend
    pub fn new() -> Result<Self, InitError> {
        #[cfg(target_os = "linux")]
        let poll_fd = epoll_create1(EpollCreateFlags::EPOLL_CLOEXEC)
            .map_err(Into::into)
            .map_err(InitError::Io)?;

        #[cfg(any(
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd"
        ))]
        let poll_fd = kqueue().map_err(Into::into).map_err(InitError::Io)?;

        Ok(Backend { handle: Handle::new(), poll_fd })
    }

    /// Initializes a connection to a client.
    ///
    /// The `data` parameter contains data that will be associated with the client.
    pub fn insert_client(
        &mut self,
        stream: UnixStream,
        data: Arc<dyn ClientData<D>>,
    ) -> std::io::Result<ClientId> {
        let client_fd = stream.as_raw_fd();
        let id = self.handle.clients.create_client(stream, data);

        // register the client to the internal epoll
        #[cfg(target_os = "linux")]
        let ret = {
            let mut evt = EpollEvent::new(EpollFlags::EPOLLIN, id.as_u64());
            epoll_ctl(self.poll_fd, EpollOp::EpollCtlAdd, client_fd, &mut evt)
        };

        #[cfg(any(
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd"
        ))]
        let ret = {
            let evt = KEvent::new(
                client_fd as usize,
                EventFilter::EVFILT_READ,
                EventFlag::EV_ADD | EventFlag::EV_RECEIPT,
                FilterFlag::empty(),
                0,
                id.as_u64() as isize,
            );

            kevent_ts(self.poll_fd, &[evt], &mut [], None).map(|_| ())
        };

        match ret {
            Ok(()) => Ok(id),
            Err(e) => {
                self.handle.kill_client(id, DisconnectReason::ConnectionClosed);
                Err(e.into())
            }
        }
    }

    /// Flushes pending events destined for a client.
    ///
    /// If no client is specified, all pending events are flushed to all clients.
    pub fn flush(&mut self, client: Option<ClientId>) -> std::io::Result<()> {
        self.handle.flush(client)
    }

    /// Returns a handle which represents the server side state of the backend.
    ///
    /// The handle provides a variety of functionality, such as querying information about wayland objects,
    /// obtaining data associated with a client and it's objects, and creating globals.
    pub fn handle(&mut self) -> &mut Handle<D> {
        &mut self.handle
    }

    /// Returns the underlying file descriptor.
    ///
    /// The file descriptor may be monitored for activity with a polling mechanism such as epoll or kqueue.
    /// When it becomes readable, this means there are pending messages that would be dispatched if you call
    /// [`Backend::dispatch_all_clients`].
    ///
    /// The file descriptor should not be used for any other purpose than monitoring it.
    pub fn poll_fd(&self) -> RawFd {
        self.poll_fd
    }

    /// Dispatches all pending messages from the specified client.
    ///
    /// This method will not block if there are no pending messages.
    ///
    /// The provided `data` will be provided to the handler of messages received from the client.
    ///
    /// For performance reasons, use of this function should be integrated with an event loop, monitoring the
    /// file descriptor associated with the client and only calling this method when messages are available.
    pub fn dispatch_client(&mut self, data: &mut D, client_id: ClientId) -> std::io::Result<usize> {
        let ret = self.handle.dispatch_events_for(data, client_id);
        self.handle.cleanup(data);
        ret
    }

    /// Dispatches all pending messages from all clients.
    ///
    /// This method will not block if there are no pending messages.
    ///
    /// The provided `data` will be provided to the handler of messages received from the clients.
    ///
    /// For performance reasons, use of this function should be integrated with an event loop, monitoring the
    /// file descriptor retrieved by [`Backend::poll_fd`] and only calling this method when messages are
    /// available.
    #[cfg(target_os = "linux")]
    pub fn dispatch_all_clients(&mut self, data: &mut D) -> std::io::Result<usize> {
        let mut dispatched = 0;
        loop {
            let mut events = [EpollEvent::empty(); 32];
            let nevents = epoll_wait(self.poll_fd, &mut events, 0)?;

            if nevents == 0 {
                break;
            }

            for event in events.iter().take(nevents) {
                let id = ClientId::from_u64(event.data());
                // remove the cb while we call it, to gracefully handle reentrancy
                if let Ok(count) = self.handle.dispatch_events_for(data, id) {
                    dispatched += count;
                }
            }
            self.handle.cleanup(data);
        }

        Ok(dispatched)
    }

    /// Dispatches all pending messages from all clients.
    ///
    /// This method will not block if there are no pending messages.
    ///
    /// The provided `data` will be provided to the handler of messages received from the clients.
    ///
    /// For performance reasons, use of this function should be integrated with an event loop, monitoring the
    /// file descriptor retrieved by [`Backend::poll_fd`] and only calling this method when messages are
    /// available.
    #[cfg(any(
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    pub fn dispatch_all_clients(&mut self, data: &mut D) -> std::io::Result<usize> {
        let mut dispatched = 0;
        loop {
            let mut events = [KEvent::new(
                0,
                EventFilter::EVFILT_READ,
                EventFlag::empty(),
                FilterFlag::empty(),
                0,
                0,
            ); 32];

            let nevents = kevent(self.poll_fd, &[], &mut events, 0)?;

            if nevents == 0 {
                break;
            }

            for event in events.iter().take(nevents) {
                let id = ClientId::from_u64(event.udata() as u64);
                // remove the cb while we call it, to gracefully handle reentrancy
                if let Ok(count) = self.handle.dispatch_events_for(data, id) {
                    dispatched += count;
                }
            }
            self.handle.cleanup(data);
        }

        Ok(dispatched)
    }
}
