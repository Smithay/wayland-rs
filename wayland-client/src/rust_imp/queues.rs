use std::cell::Cell;
use std::collections::VecDeque;
use std::io;
use std::os::unix::io::AsRawFd;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use nix::poll::{poll, PollFlags, PollFd};

use wayland_commons::map::ObjectMap;
use wayland_commons::utils::UserData;
use wayland_commons::wire::Message;

use super::connection::{Connection, Error as CError};
use super::proxy::{ObjectMeta, ProxyInner};

pub(crate) type QueueBuffer = Arc<Mutex<VecDeque<Message>>>;

pub(crate) fn create_queue_buffer() -> QueueBuffer {
    Arc::new(Mutex::new(VecDeque::new()))
}

pub(crate) struct EventQueueInner {
    pub(crate) connection: Arc<Mutex<Connection>>,
    pub(crate) map: Arc<Mutex<ObjectMap<ObjectMeta>>>,
    pub(crate) buffer: QueueBuffer,
    display_buffer: QueueBuffer,
}

impl EventQueueInner {
    pub(crate) fn new(connection: Arc<Mutex<Connection>>, buffer: Option<QueueBuffer>) -> EventQueueInner {
        let (map, display_buffer) = {
            let cx = connection.lock().unwrap();
            (cx.map.clone(), cx.display_buffer.clone())
        };
        EventQueueInner {
            connection,
            map,
            buffer: buffer.unwrap_or_else(create_queue_buffer),
            display_buffer,
        }
    }

    #[cfg(feature = "eventloop")]
    pub(crate) fn get_connection_fd(&self) -> ::std::os::unix::io::RawFd {
        self.connection.lock().unwrap().socket.get_socket().as_raw_fd()
    }

    pub(crate) fn dispatch(&self) -> io::Result<u32> {
        // don't read events if there are some pending
        if let Err(()) = self.prepare_read() {
            return self.dispatch_pending();
        }

        // temporarily retrieve the socket Fd, only using it for POLL-ing!
        let socket_fd;
        {
            // Flush the outgoing socket
            let mut conn_lock = self.connection.lock().unwrap();
            socket_fd = conn_lock.socket.get_socket().as_raw_fd();
            loop {
                match conn_lock.flush() {
                    Ok(_) => break,
                    Err(::nix::Error::Sys(::nix::errno::Errno::EAGAIN)) => {
                        // EAGAIN, we need to wait before writing, so we poll the socket
                        let poll_ret = poll(&mut [PollFd::new(socket_fd, PollFlags::POLLOUT)], -1);
                        match poll_ret {
                            Ok(_) => continue,
                            Err(::nix::Error::Sys(e)) => {
                                self.cancel_read();
                                return Err(e.into());
                            }
                            Err(_) => unreachable!(),
                        }
                    }
                    Err(::nix::Error::Sys(e)) => {
                        if e != ::nix::errno::Errno::EPIPE {
                            // don't abort on EPIPE, so we can continue reading
                            // to get the protocol error
                            self.cancel_read();
                            return Err(e.into());
                        }
                    }
                    Err(_) => unreachable!(),
                }
            }
        }

        // wait for incoming messages to arrive
        match poll(&mut [PollFd::new(socket_fd, PollFlags::POLLIN)], -1) {
            Ok(_) => (),
            Err(::nix::Error::Sys(e)) => {
                self.cancel_read();
                return Err(e.into());
            }
            Err(_) => unreachable!(),
        }

        match self.read_events() {
            Ok(_) => (),
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                // we waited for read readiness be then received a WouldBlock error
                // this means that an other thread was also reading events and read them
                // under our nose
                // this is alright, continue
            }
            Err(e) => return Err(e),
        }

        self.dispatch_pending()
    }

    fn dispatch_buffer(&self, buffer: &mut VecDeque<Message>) -> io::Result<u32> {
        let mut count = 0;
        let mut proxymap = super::ProxyMap::make(self.map.clone(), self.connection.clone());
        for msg in buffer.drain(..) {
            let id = msg.sender_id;
            if let Some(proxy) = ProxyInner::from_id(id, self.map.clone(), self.connection.clone()) {
                let object = proxy.object.clone();
                let mut dispatcher = object.meta.dispatcher.lock().unwrap();
                if let Err(()) = dispatcher.dispatch(msg, proxy, &mut proxymap) {
                    return Err(io::Error::new(
                        io::ErrorKind::Other,
                        format!("Dispatch for object {}@{} errored.", object.interface, id),
                    ));
                } else {
                    count += 1;
                }
            } else {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("Received an event for unknown object {}.", id),
                ));
            }
        }
        Ok(count)
    }

    pub(crate) fn dispatch_pending(&self) -> io::Result<u32> {
        // First always dispatch the display buffer
        let display_dispatched = {
            let mut buffer = self.display_buffer.lock().unwrap();
            self.dispatch_buffer(&mut *buffer)
        }?;

        // Then our actual buffer
        let self_dispatched = {
            let mut buffer = self.buffer.lock().unwrap();
            self.dispatch_buffer(&mut *buffer)
        }?;

        Ok(display_dispatched + self_dispatched)
    }

    pub(crate) fn sync_roundtrip(&self) -> io::Result<u32> {
        use protocol::wl_callback::{Event as CbEvent, WlCallback};
        use protocol::wl_display::{RequestsTrait as DisplayRequests, WlDisplay};
        use Proxy;
        // first retrieve the display and make a wrapper for it in this event queue
        let display: Proxy<WlDisplay> = Proxy::wrap(
            ProxyInner::from_id(1, self.map.clone(), self.connection.clone())
                .unwrap()
                .make_wrapper(self)
                .unwrap(),
        );

        let done = Rc::new(Cell::new(false));
        let ret = display.sync(|np| {
            Proxy::wrap(unsafe {
                let done2 = done.clone();
                np.inner.implement::<WlCallback, _>(
                    move |CbEvent::Done { .. }, _| {
                        done2.set(true);
                    },
                    UserData::empty(),
                )
            })
        });

        if let Err(()) = ret {
            return Err(::nix::errno::Errno::EPROTO.into());
        }

        let mut dispatched = 0;

        loop {
            dispatched += self.dispatch()?;
            if done.get() {
                return Ok(dispatched);
            }
        }
    }

    pub(crate) fn prepare_read(&self) -> Result<(), ()> {
        if !self.buffer.lock().unwrap().is_empty() {
            return Err(());
        }

        // TODO: un-mock
        Ok(())
    }

    pub(crate) fn read_events(&self) -> io::Result<i32> {
        // TODO: integrate more properly with prepare read with a fence
        match self.connection.lock().unwrap().read_events() {
            Ok(n) => Ok(n as i32),
            Err(CError::Protocol) => Err(::nix::errno::Errno::EPROTO.into()),
            Err(CError::Parse(_)) => Err(::nix::errno::Errno::EPROTO.into()),
            Err(CError::Nix(::nix::Error::Sys(errno))) => Err(errno.into()),
            Err(CError::Nix(_)) => unreachable!(),
        }
    }

    pub(crate) fn cancel_read(&self) {
        // TODO: un-mock
    }
}
