use std::cell::Cell;
use std::collections::VecDeque;
use std::io;
use std::os::unix::io::AsRawFd;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use nix::poll::{poll, PollFd, PollFlags};

use wayland_commons::map::ObjectMap;
use wayland_commons::wire::{Argument, Message};

use super::connection::{Connection, Error as CError};
use super::proxy::{ObjectMeta, ProxyInner};
use super::Dispatched;

use crate::{AnonymousObject, DispatchData, Filter, Main, RawEvent};

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
    pub(crate) fn new(
        connection: Arc<Mutex<Connection>>,
        buffer: Option<QueueBuffer>,
    ) -> EventQueueInner {
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

    pub(crate) fn dispatch<F>(&self, mut data: DispatchData, mut fallback: F) -> io::Result<u32>
    where
        F: FnMut(RawEvent, Main<AnonymousObject>, DispatchData<'_>),
    {
        // don't read events if there are some pending
        if let Err(()) = self.prepare_read() {
            return self.dispatch_pending(data.reborrow(), &mut fallback);
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
                    Err(nix::Error::EAGAIN) => {
                        // EAGAIN, we need to wait before writing, so we poll the socket
                        let poll_ret = poll(&mut [PollFd::new(socket_fd, PollFlags::POLLOUT)], -1);
                        match poll_ret {
                            Ok(_) => continue,
                            Err(e) => {
                                self.cancel_read();
                                return Err(e.into());
                            }
                        }
                    }
                    Err(e) => {
                        if e != nix::Error::EPIPE {
                            // don't abort on EPIPE, so we can continue reading
                            // to get the protocol error
                            self.cancel_read();
                            return Err(e.into());
                        }
                    }
                }
            }
        }

        // wait for incoming messages to arrive
        match poll(&mut [PollFd::new(socket_fd, PollFlags::POLLIN)], -1) {
            Ok(_) => (),
            Err(e) => {
                self.cancel_read();
                return Err(e.into());
            }
        }
        let read_ret = self.read_events();

        // even if read_events returned an error, it may have queued messages the need dispatching
        // so we dispatch them
        let dispatch_ret = self.dispatch_pending(data.reborrow(), &mut fallback);

        match read_ret {
            Ok(()) => (),
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                // we waited for read readiness be then received a WouldBlock error
                // this means that an other thread was also reading events and read them
                // under our nose
                // this is alright, continue
            }
            Err(e) => return Err(e),
        }

        dispatch_ret
    }

    fn dispatch_buffer<F>(
        &self,
        buffer: &Mutex<VecDeque<Message>>,
        mut data: DispatchData,
        mut fallback: F,
    ) -> io::Result<u32>
    where
        F: FnMut(RawEvent, Main<AnonymousObject>, DispatchData<'_>),
    {
        let mut count = 0;
        let mut proxymap = super::ProxyMap::make(self.map.clone(), self.connection.clone());
        loop {
            let msg = { buffer.lock().unwrap().pop_front() };
            let msg = match msg {
                Some(m) => m,
                None => break,
            };
            let id = msg.sender_id;
            if let Some(proxy) = ProxyInner::from_id(id, self.map.clone(), self.connection.clone())
            {
                let object = proxy.object.clone();
                if object.meta.client_destroyed {
                    // This is a potential race, if we reach here it means that the proxy was
                    // destroyed by the user between this message was queued and now. To handle it
                    // correctly, we must close any FDs it contains, mark any child object as
                    // destroyed (but the server will never know about it, so the ids will be
                    // leaked) and discard the event.
                    for arg in msg.args {
                        match arg {
                            Argument::Fd(fd) => {
                                let _ = ::nix::unistd::close(fd);
                            }
                            Argument::NewId(id) => {
                                let mut map = self.map.lock().unwrap();
                                map.with(id, |obj| {
                                    obj.meta.client_destroyed = true;
                                })
                                .unwrap();
                            }
                            _ => {}
                        }
                    }
                    continue;
                }
                let mut dispatcher = object.meta.dispatcher.lock().unwrap();
                match dispatcher.dispatch(msg, proxy, &mut proxymap, data.reborrow()) {
                    Dispatched::Yes => {
                        count += 1;
                    }
                    Dispatched::NoDispatch(msg, proxy) => {
                        let raw_event = message_to_rawevent(msg, &proxy, &mut proxymap);
                        fallback(raw_event, Main::wrap(proxy), data.reborrow());
                        count += 1;
                    }
                    Dispatched::BadMsg => {
                        return Err(io::Error::new(
                            io::ErrorKind::Other,
                            format!("Dispatch for object {}@{} errored.", object.interface, id),
                        ))
                    }
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

    pub(crate) fn dispatch_pending<F>(&self, mut data: DispatchData, fallback: F) -> io::Result<u32>
    where
        F: FnMut(RawEvent, Main<AnonymousObject>, DispatchData<'_>),
    {
        // First always dispatch the display buffer
        let display_dispatched =
            self.dispatch_buffer(&self.display_buffer, data.reborrow(), |_, _, _| unreachable!())?;

        // Then our actual buffer
        let self_dispatched = self.dispatch_buffer(&self.buffer, data.reborrow(), fallback)?;

        Ok(display_dispatched + self_dispatched)
    }

    pub(crate) fn sync_roundtrip<F>(
        &self,
        mut data: DispatchData,
        mut fallback: F,
    ) -> io::Result<u32>
    where
        F: FnMut(RawEvent, Main<AnonymousObject>, DispatchData<'_>),
    {
        use crate::protocol::wl_callback::{Event as CbEvent, WlCallback};
        use crate::protocol::wl_display::{Request as DRequest, WlDisplay};
        // first retrieve the display and make a wrapper for it in this event queue
        let mut display =
            ProxyInner::from_id(1, self.map.clone(), self.connection.clone()).unwrap();
        display.attach(self);

        let done = Rc::new(Cell::new(false));
        let cb = display.send::<WlDisplay, WlCallback>(DRequest::Sync {}, Some(1)).unwrap();
        let done2 = done.clone();
        cb.assign::<WlCallback, _>(Filter::new(move |(_, CbEvent::Done { .. }), _, _| {
            done2.set(true);
        }));

        let mut dispatched = 0;

        loop {
            dispatched += self.dispatch(data.reborrow(), &mut fallback)?;
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

    pub(crate) fn read_events(&self) -> io::Result<()> {
        // TODO: integrate more properly with prepare read with a fence
        match self.connection.lock().unwrap().read_events() {
            Ok(_) => Ok(()),
            Err(CError::Protocol(e)) => {
                eprintln!("[wayland-client] Protocol error while reading events: {}", e);
                Err(::nix::errno::Errno::EPROTO.into())
            }
            Err(CError::Parse(e)) => {
                eprintln!("[wayland-client] Parse error while reading events: {}", e);
                Err(::nix::errno::Errno::EPROTO.into())
            }
            Err(CError::Nix(errno)) => Err(errno.into()),
        }
    }

    pub(crate) fn cancel_read(&self) {
        // TODO: un-mock
    }
}

fn message_to_rawevent(msg: Message, proxy: &ProxyInner, map: &mut super::ProxyMap) -> RawEvent {
    let Message { opcode, args, .. } = msg;

    let args = args
        .into_iter()
        .map(|a| match a {
            Argument::Int(i) => crate::Argument::Int(i),
            Argument::Uint(u) => crate::Argument::Uint(u),
            Argument::Array(v) => {
                crate::Argument::Array(if v.is_empty() { None } else { Some(*v) })
            }
            Argument::Fixed(f) => crate::Argument::Float((f as f32) / 256.),
            Argument::Fd(f) => crate::Argument::Fd(f),
            Argument::Str(cs) => crate::Argument::Str({
                let bytes = cs.into_bytes();
                if bytes.is_empty() {
                    None
                } else {
                    Some(
                        String::from_utf8(bytes)
                            .unwrap_or_else(|e| String::from_utf8_lossy(&e.into_bytes()).into()),
                    )
                }
            }),
            Argument::Object(id) => crate::Argument::Object(map.get(id)),
            Argument::NewId(id) => crate::Argument::NewId(map.get_new(id)),
        })
        .collect();

    RawEvent {
        interface: proxy.object.interface,
        opcode,
        name: proxy.object.events[opcode as usize].name,
        args,
    }
}
