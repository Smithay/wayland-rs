use std::cell::RefCell;
use std::os::unix::io::{FromRawFd, RawFd};
use std::sync::{Arc, Mutex};

use nix::Result as NixResult;

use wayland_commons::map::{Object, ObjectMap};
use wayland_commons::socket::{BufferedSocket, Socket};
use wayland_commons::wire::{Argument, ArgumentType, Message, MessageParseError};

use super::proxy::ObjectMeta;
use super::queues::QueueBuffer;

#[derive(Clone, Debug)]
pub(crate) enum Error {
    Protocol,
    Parse(MessageParseError),
    Nix(::nix::Error),
}

pub(crate) struct Connection {
    pub(crate) socket: BufferedSocket,
    pub(crate) map: Arc<Mutex<ObjectMap<ObjectMeta>>>,
    pub(crate) last_error: Arc<Mutex<Option<Error>>>,
    pub(crate) display_buffer: QueueBuffer,
}

impl Connection {
    pub(crate) unsafe fn new(fd: RawFd, display_object: Object<ObjectMeta>) -> Connection {
        let socket = BufferedSocket::new(Socket::from_raw_fd(fd));

        let mut map = ObjectMap::new();
        // Insert first pre-existing object
        let display_buffer = display_object.meta.buffer.clone();
        map.insert_at(1, display_object).unwrap();

        Connection {
            socket,
            map: Arc::new(Mutex::new(map)),
            last_error: Arc::new(Mutex::new(None)),
            display_buffer,
        }
    }

    pub(crate) fn write_message(&mut self, msg: &Message) -> NixResult<()> {
        self.socket.write_message(msg)
    }

    pub(crate) fn flush(&mut self) -> NixResult<()> {
        self.socket.flush()
    }

    pub(crate) fn read_events(&mut self) -> Result<usize, Error> {
        if let Some(ref err) = *self.last_error.lock().unwrap() {
            return Err(err.clone());
        }
        // acquire the map lock, this means no objects can be created nor destroyed while we
        // are reading events
        let mut map = self.map.lock().unwrap();
        // wrap it in a RefCell for cheap sharing in the two closures below
        let map = RefCell::new(&mut *map);
        let mut last_error = self.last_error.lock().unwrap();
        // read messages
        let ret = self.socket.read_messages(
            |id, opcode| {
                map.borrow()
                    .find(id)
                    .and_then(|o| o.events.get(opcode as usize))
                    .map(|desc| desc.signature)
            },
            |msg| {
                let mut map = map.borrow_mut();
                let object = match map.find(msg.sender_id) {
                    Some(obj) => obj,
                    None => {
                        // this is a message sent to a destroyed object
                        // to avoid dying because of races, we just consume it into void
                        // closing any associated FDs
                        for a in msg.args {
                            if let Argument::Fd(fd) = a {
                                let _ = ::nix::unistd::close(fd);
                            }
                        }
                        // continue parsing to the next message
                        return true;
                    }
                };

                // create a new object if applicable
                if let Some(child) = object.event_child(msg.opcode) {
                    let new_id = msg
                        .args
                        .iter()
                        .flat_map(|a| {
                            if let &Argument::NewId(nid) = a {
                                Some(nid)
                            } else {
                                None
                            }
                        })
                        .next()
                        .unwrap();
                    let child_interface = child.interface;
                    if let Err(()) = map.insert_at(new_id, child) {
                        eprintln!(
                            "[wayland-client] Protocol error: server tried to create an object \"{}\" with invalid id \"{}\".",
                            child_interface,
                            new_id
                        );
                        // abort parsing, this is an unrecoverable error
                        *last_error = Some(Error::Protocol);
                        return false;
                    }
                } else {
                    // debug assert: if this opcode does not define a child, then there should be no
                    // NewId argument
                    debug_assert!(msg.args.iter().any(|a| a.get_type() == ArgumentType::NewId) == false);
                }

                // send the message to the appropriate pending queue
                object.meta.buffer.lock().unwrap().push_back(msg);
                // continue parsing
                true
            },
        );
        match ret {
            Ok(Ok(n)) => {
                if let Some(ref e) = *last_error {
                    Err(e.clone())
                } else {
                    Ok(n)
                }
            }
            Ok(Err(e)) => {
                *last_error = Some(Error::Parse(e.clone()));
                Err(Error::Parse(e))
            }
            // non-fatal error
            Err(e @ ::nix::Error::Sys(::nix::errno::Errno::EAGAIN)) => Err(Error::Nix(e)),
            // fatal errors
            Err(e) => {
                *last_error = Some(Error::Nix(e));
                Err(Error::Nix(e))
            }
        }
    }
}
