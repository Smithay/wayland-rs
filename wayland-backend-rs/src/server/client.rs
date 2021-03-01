use std::{
    os::unix::io::{FromRawFd, RawFd},
    sync::Arc,
};

use wayland_commons::{
    core_interfaces::{
        ANONYMOUS_INTERFACE, WL_CALLBACK_INTERFACE, WL_DISPLAY_INTERFACE, WL_REGISTRY_INTERFACE,
    },
    server::{InvalidId, ObjectData, ServerBackend},
    Argument, Interface, ObjectInfo,
};

use smallvec::SmallVec;

use crate::{
    map::{Object, ObjectMap},
    same_interface,
    socket::{BufferedSocket, Socket},
    wire::{check_for_signature, Message, MessageParseError, INLINE_ARGS},
};

use super::{registry::Registry, ClientId, Data, GlobalId, ObjectId};

pub(crate) struct Client<B> {
    socket: BufferedSocket,
    map: ObjectMap<Data<B>>,
    debug: bool,
    last_serial: u32,
    pub(crate) id: ClientId,
    pub(crate) killed: bool,
}

impl<B> Client<B> {
    fn next_serial(&mut self) -> u32 {
        self.last_serial = self.last_serial.wrapping_add(1);
        self.last_serial
    }
}

impl<B: ServerBackend<ObjectId = ObjectId, ClientId = ClientId, GlobalId = GlobalId>> Client<B> {
    pub(crate) unsafe fn new(fd: RawFd, id: ClientId, debug: bool) -> Self {
        let socket = BufferedSocket::new(Socket::from_raw_fd(fd));
        let mut map = ObjectMap::new();
        map.insert_at(
            1,
            Object {
                interface: &WL_DISPLAY_INTERFACE,
                version: 1,
                data: Data { user_data: Arc::new(DumbObjectData), serial: 0 },
            },
        )
        .unwrap();

        Client { socket, map, debug, id, killed: false, last_serial: 0 }
    }

    pub(crate) fn create_object(
        &mut self,
        interface: &'static Interface,
        version: u32,
        user_data: Arc<dyn ObjectData<B>>,
    ) -> u32 {
        let serial = self.next_serial();
        self.map.server_insert_new(Object { interface, version, data: Data { serial, user_data } })
    }

    pub(crate) fn object_info(&self, id: ObjectId) -> Result<ObjectInfo, InvalidId> {
        let object = self.get_object(id)?;
        Ok(ObjectInfo { id: id.id, interface: object.interface, version: object.version })
    }

    pub(crate) fn send_event(
        &mut self,
        object_id: ObjectId,
        opcode: u16,
        args: &[Argument<ObjectId>],
    ) -> Result<(), InvalidId> {
        if self.killed {
            return Ok(());
        }
        let object = self.get_object(object_id)?;

        let message_desc = match object.interface.events.get(opcode as usize) {
            Some(msg) => msg,
            None => {
                panic!(
                    "Unknown opcode {} for object {}@{}.",
                    opcode, object.interface.name, object_id.id
                );
            }
        };

        if !check_for_signature(message_desc.signature, args) {
            panic!(
                "Unexpected signature for event {}@{}.{}: expected {:?}, got {:?}.",
                object.interface.name,
                object_id.id,
                message_desc.name,
                message_desc.signature,
                args
            );
        }

        if self.debug {
            crate::debug::print_send_message(
                object.interface.name,
                object_id.id,
                message_desc.name,
                &args,
            );
        }

        let mut msg_args = SmallVec::with_capacity(args.len());
        let mut arg_interfaces = message_desc.arg_interfaces.iter();
        for arg in args.iter().cloned() {
            msg_args.push(match arg {
                Argument::Array(a) => Argument::Array(a),
                Argument::Int(i) => Argument::Int(i),
                Argument::Uint(u) => Argument::Uint(u),
                Argument::Str(s) => Argument::Str(s),
                Argument::Fixed(f) => Argument::Fixed(f),
                Argument::Fd(f) => Argument::Fd(f),
                Argument::NewId(o) => {
                    let object = self.get_object(o)?;
                    let child_interface = match message_desc.child_interface {
                        Some(iface) => iface,
                        None => panic!("Trying to send event {}@{}.{} which creates an object without specifying its interface, this is unsupported.", object_id.interface.name, object_id.id, message_desc.name),
                    };
                    if !same_interface(child_interface, object.interface) {
                        panic!("Event {}@{}.{} expects an argument of interface {} but {} was provided instead.", object.interface.name, object_id.id, message_desc.name, child_interface.name, object.interface.name);
                    }
                    Argument::Object(o.id)
                },
                Argument::Object(o) => {
                    let object = self.get_object(o)?;
                    let next_interface = arg_interfaces.next().unwrap();
                    if !same_interface(next_interface, object.interface) {
                        panic!("Event {}@{}.{} expects an argument of interface {} but {} was provided instead.", object.interface.name, object_id.id, message_desc.name, next_interface.name, object.interface.name);
                    }
                    Argument::Object(o.id)
                }
            });
        }

        let msg = Message { sender_id: object_id.id, opcode, args: msg_args };

        if let Err(err) = self.socket.write_message(&msg) {
            self.killed = true;
        }

        // Handle destruction if relevant
        if message_desc.is_destructor {
            self.map.remove(object_id.id);
            object.data.user_data.destroyed(self.id, object_id);
        }

        Ok(())
    }

    pub(crate) fn get_object_data(
        &self,
        id: ObjectId,
    ) -> Result<Arc<dyn ObjectData<B>>, InvalidId> {
        let object = self.get_object(id)?;
        Ok(object.data.user_data)
    }

    pub(crate) fn post_error(mut self, object_id: ObjectId, error_code: u32, message: String) {
        todo!()
    }

    pub(crate) fn flush(&mut self) -> std::io::Result<()> {
        self.socket.flush()
    }

    fn all_objects<'a>(&'a self) -> impl Iterator<Item = ObjectId> + 'a {
        let client_id = self.id.id;
        self.map.all_objects().map(move |(id, obj)| ObjectId {
            id,
            client_id,
            interface: obj.interface,
            serial: obj.data.serial,
        })
    }

    pub(crate) fn next_request(&mut self) -> std::io::Result<(Message, Object<Data<B>>)> {
        if self.killed {
            return Err(nix::errno::Errno::EPIPE.into());
        }
        loop {
            let map = &self.map;
            let msg = match self.socket.read_one_message(|id, opcode| {
                map.find(id)
                    .and_then(|o| o.interface.events.get(opcode as usize))
                    .map(|desc| desc.signature)
            }) {
                Ok(msg) => msg,
                Err(MessageParseError::MissingData) | Err(MessageParseError::MissingFD) => {
                    // need to read more data
                    if let Err(e) = self.socket.fill_incoming_buffers() {
                        if e.kind() != std::io::ErrorKind::WouldBlock {
                            self.killed = true;
                        }
                        return Err(e);
                    }
                    continue;
                }
                Err(MessageParseError::Malformed) => {
                    self.killed = true;
                    return Err(nix::errno::Errno::EPROTO.into());
                }
            };

            let obj = self.map.find(msg.sender_id).unwrap();
            return Ok((msg, obj));
        }
    }

    fn get_object(&self, id: ObjectId) -> Result<Object<Data<B>>, InvalidId> {
        let object = self.map.find(id.id).ok_or(InvalidId)?;
        if object.data.serial != id.serial {
            return Err(InvalidId);
        }
        Ok(object)
    }

    fn handle_display_request(&mut self, message: Message) {
        match message.opcode {
            // wl_display.sync(new id wl_callback)
            0 => {
                if let &[Argument::NewId(new_id)] = &message.args[..] {
                    let serial = self.next_serial();
                    let callback_obj = Object {
                        interface: &WL_CALLBACK_INTERFACE,
                        version: 1,
                        data: Data { user_data: Arc::new(DumbObjectData), serial },
                    };
                    if let Err(()) = self.map.insert_at(new_id, callback_obj) {
                        self.killed = true;
                        return;
                    }
                    // send wl_callback.done(0)
                    self.send_event(
                        ObjectId {
                            id: new_id,
                            client_id: self.id.id,
                            serial,
                            interface: &WL_CALLBACK_INTERFACE,
                        },
                        0,
                        &[Argument::Uint(0)],
                    )
                    .unwrap();
                } else {
                    unreachable!()
                }
            }
            // wl_display.get_registry(new id wl_registry)
            1 => {
                if let &[Argument::NewId(new_id)] = &message.args[..] {
                    let serial = self.next_serial();
                    let callback_obj = Object {
                        interface: &WL_REGISTRY_INTERFACE,
                        version: 1,
                        data: Data { user_data: Arc::new(DumbObjectData), serial },
                    };
                    if let Err(()) = self.map.insert_at(new_id, callback_obj) {
                        self.killed = true;
                        return;
                    }
                } else {
                    unreachable!()
                }
            }
            _ => {
                // unkown opcode, kill the client
                self.killed = true;
            }
        }
    }

    fn handle_registry_request(&mut self, message: Message, registry: &mut Registry<B>) {
        match message.opcode {
            // wl_registry.bind(uint name, str interface, uint version, new id)
            0 => {
                if let &[Argument::Uint(name), Argument::Str(ref interface_name), Argument::Uint(version), Argument::NewId(new_id)] =
                    &message.args[..]
                {
                    if let Some((interface, global_id, handler)) =
                        registry.check_bind(self.id, name, interface_name, version)
                    {
                        let user_data = handler.clone().make_data(&ObjectInfo {
                            id: new_id,
                            interface,
                            version,
                        });
                        let serial = self.next_serial();
                        let object =
                            Object { interface, version, data: Data { serial, user_data } };
                        if let Err(()) = self.map.insert_at(new_id, object) {
                            self.killed = true;
                            return;
                        }
                        handler.bind(
                            self.id,
                            global_id,
                            ObjectId { id: new_id, client_id: self.id.id, interface, serial },
                        );
                    } else {
                        self.killed = true;
                    }
                } else {
                    unreachable!()
                }
            }
            _ => {
                // unkown opcode, kill the client
                self.killed = true;
            }
        }
    }

    fn process_request(
        &mut self,
        object: Object<Data<B>>,
        message: Message,
    ) -> Option<SmallVec<[Argument<ObjectId>; INLINE_ARGS]>> {
        let message_desc = object.interface.events.get(message.opcode as usize).unwrap();
        // Convert the arguments and create the new object if applicable
        let mut new_args =
            SmallVec::<[Argument<ObjectId>; INLINE_ARGS]>::with_capacity(message.args.len());
        let mut arg_interfaces = message_desc.arg_interfaces.iter();
        for arg in message.args.into_iter() {
            new_args.push(match arg {
                Argument::Array(a) => Argument::Array(a),
                Argument::Int(i) => Argument::Int(i),
                Argument::Uint(u) => Argument::Uint(u),
                Argument::Str(s) => Argument::Str(s),
                Argument::Fixed(f) => Argument::Fixed(f),
                Argument::Fd(f) => Argument::Fd(f),
                Argument::Object(o) => {
                    // Lookup the object to make the appropriate Id
                    let obj = match self.map.find(o) {
                        Some(o) => o,
                        None => {
                            self.killed = true;
                            return None;
                        }
                    };
                    if let Some(next_interface) = arg_interfaces.next() {
                        if !same_interface(next_interface, obj.interface) && !same_interface(next_interface, &ANONYMOUS_INTERFACE){
                            self.killed = true;
                            return None;
                        }
                    }
                    Argument::Object(ObjectId { id: o, client_id: self.id.id, serial: obj.data.serial, interface: obj.interface })
                }
                Argument::NewId(new_id) => {
                    // An object should be created
                    let child_interface = match message_desc.child_interface {
                        Some(iface) => iface,
                        None => panic!("Received request {}@{}.{} which creates an object without specifying its interface, this is unsupported.", object.interface.name, message.sender_id, message_desc.name),
                    };

                    let child_udata = object.data.user_data.clone().make_child(&ObjectInfo {
                        id: new_id,
                        interface: child_interface,
                        version: object.version
                    });

                    let child_obj = Object {
                        interface: child_interface,
                        version: object.version,
                        data: Data {
                            user_data: child_udata,
                            serial: self.next_serial(),
                        }
                    };

                    let child_id = ObjectId { id: new_id, client_id: self.id.id, serial: child_obj.data.serial, interface: child_obj.interface };

                    if let Err(()) = self.map.insert_at(new_id, child_obj) {
                        // abort parsing, this is an unrecoverable error
                        self.killed = true;
                        return None;
                    }

                    Argument::NewId(child_id)
                }
            });
        }
        Some(new_args)
    }
}

struct DumbObjectData;

impl<B: ServerBackend> ObjectData<B> for DumbObjectData {
    fn make_child(self: Arc<Self>, _child_info: &ObjectInfo) -> Arc<dyn ObjectData<B>> {
        unreachable!()
    }

    fn request(
        &self,
        _handle: &mut B::Handle,
        _client_id: B::ClientId,
        _object_id: B::ObjectId,
        _opcode: u16,
        _arguments: &[Argument<B::ObjectId>],
    ) {
        unreachable!()
    }

    fn destroyed(&self, _client_id: B::ClientId, _object_id: B::ObjectId) {
        unreachable!()
    }
}
