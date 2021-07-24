use std::cell::RefCell;
use std::os::unix::io::{FromRawFd, RawFd};
use std::sync::{Arc, Mutex};

use nix::Result as NixResult;

use wayland_commons::map::{Object, ObjectMap, SERVER_ID_LIMIT};
use wayland_commons::socket::{BufferedSocket, Socket};
use wayland_commons::wire::{Argument, ArgumentType, Message, MessageParseError};

use super::proxy::ObjectMeta;
use super::queues::QueueBuffer;

use crate::ProtocolError;

#[derive(Clone, Debug)]
pub(crate) enum Error {
    Protocol(ProtocolError),
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
                // Early exit on protocol error
                if msg.sender_id == 1 && msg.opcode == 0 {
                    if let [Argument::Object(faulty_id), Argument::Uint(error_code), Argument::Str(ref error_msg)] = &msg.args[..] {
                        let error_msg = error_msg.to_string_lossy().into_owned();
                        let faulty_interface = map.borrow().find(*faulty_id).map(|obj| obj.interface).unwrap_or("unknown");
                        // abort parsing, this is an unrecoverable error
                        *last_error = Some(Error::Protocol(ProtocolError {
                            code: *error_code,
                            object_id: *faulty_id,
                            object_interface: faulty_interface,
                            message: error_msg
                        }));
                    } else {
                        unreachable!();
                    }
                    return false;
                }

                // dispatch the message to the proper object
                let mut map = map.borrow_mut();
                let object = map.find(msg.sender_id);

                // create a new object if applicable
                if let Some((mut child, dead_parent)) = object
                    .as_ref()
                    .and_then(|o| o.event_child(msg.opcode).map(|c| (c, o.meta.client_destroyed)))
                {
                    let new_id = msg
                        .args
                        .iter()
                        .flat_map(|a| if let Argument::NewId(nid) = *a { Some(nid) } else { None })
                        .next()
                        .unwrap();
                    let child_interface = child.interface;
                    // if this ID belonged to a now destroyed server object, we can replace it
                    if new_id >= SERVER_ID_LIMIT
                        && map.with(new_id, |obj| obj.meta.client_destroyed).unwrap_or(false)
                    {
                        map.remove(new_id)
                    }
                    // if the parent object is already destroyed, the user will never see this
                    // object, so we set it as client_destroyed to ignore all future messages to it
                    if dead_parent {
                        child.meta.client_destroyed = true;
                    }
                    if let Err(()) = map.insert_at(new_id, child) {
                        // abort parsing, this is an unrecoverable error
                        *last_error = Some(Error::Protocol(ProtocolError {
                            code: 0,
                            object_id: 0,
                            object_interface: "",
                            message: format!(
                                "Protocol error: server tried to create \
                                an object \"{}\" with invalid id \"{}\".",
                                child_interface, new_id
                            ),
                        }));
                        return false;
                    }
                } else {
                    // debug assert: if this opcode does not define a child, then there should be no
                    // NewId argument
                    debug_assert!(!msg.args.iter().any(|a| a.get_type() == ArgumentType::NewId));
                }

                // send the message to the appropriate pending queue
                match object {
                    Some(Object { meta: ObjectMeta { client_destroyed: true, .. }, .. }) | None => {
                        // this is a message sent to a destroyed object
                        // to avoid dying because of races, we just consume it into void
                        // closing any associated FDs
                        for a in msg.args {
                            if let Argument::Fd(fd) = a {
                                let _ = ::nix::unistd::close(fd);
                            }
                        }
                    }
                    Some(obj) => {
                        obj.meta.buffer.lock().unwrap().push_back(msg);
                    }
                };

                // continue parsing
                true
            },
        );

        if let Some(ref e) = *last_error {
            // a protocol error was generated, don't lose it, it is the source of any subsequent error
            return Err(e.clone());
        }

        match ret {
            Ok(Ok(n)) => Ok(n),
            Ok(Err(e)) => {
                *last_error = Some(Error::Parse(e.clone()));
                Err(Error::Parse(e))
            }
            // non-fatal error
            Err(e @ nix::Error::EAGAIN) => Err(Error::Nix(e)),
            // fatal errors
            Err(e) => {
                *last_error = Some(Error::Nix(e));
                Err(Error::Nix(e))
            }
        }
    }
}
