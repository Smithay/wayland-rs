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

pub struct Backend<D> {
    handle: Handle<D>,
    poll_fd: RawFd,
}

impl<D> Backend<D> {
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

    pub fn flush(&mut self, client: Option<ClientId>) -> std::io::Result<()> {
        self.handle.flush(client)
    }

    pub fn handle(&mut self) -> &mut Handle<D> {
        &mut self.handle
    }

    pub fn poll_fd(&self) -> RawFd {
        self.poll_fd
    }

    pub fn dispatch_events_for(
        &mut self,
        data: &mut D,
        client_id: ClientId,
    ) -> std::io::Result<usize> {
        let ret = self.handle.dispatch_events_for(data, client_id);
        self.handle.cleanup();
        ret
    }

    #[cfg(target_os = "linux")]
    pub fn dispatch_events(&mut self, data: &mut D) -> std::io::Result<usize> {
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
        }

        self.handle.cleanup();

        Ok(dispatched)
    }

    #[cfg(any(
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    pub fn dispatch_events(&mut self, data: &mut D) -> std::io::Result<usize> {
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

            let nevents = kevent(self.poll_fd, &[], &mut events, 0).map_err(nix_to_io)?;

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
        }

        self.handle.cleanup();

        Ok(dispatched)
    }
}
