use std::{
    ffi::CString,
    os::unix::{
        io::{FromRawFd, IntoRawFd},
        net::UnixStream,
    },
    sync::Arc,
};

use crate::{
    core_interfaces::{WL_CALLBACK_INTERFACE, WL_DISPLAY_INTERFACE, WL_REGISTRY_INTERFACE},
    protocol::{
        AllowNull, Argument, ArgumentType, Interface, Message, ObjectInfo, ProtocolError,
        ANONYMOUS_INTERFACE, INLINE_ARGS,
    },
    types::{
        check_for_signature, same_interface, same_interface_or_anonymous,
        server::{DisconnectReason, InvalidId},
    },
};

use smallvec::SmallVec;

use crate::rs::{
    map::{Object, ObjectMap},
    socket::{BufferedSocket, Socket},
    wire::MessageParseError,
};

use super::{
    registry::Registry, ClientData, ClientId, Data, GlobalHandler, GlobalId, Handle, ObjectData,
    ObjectId,
};

#[repr(u32)]
#[allow(dead_code)]
pub(crate) enum DisplayError {
    InvalidObject = 0,
    InvalidMethod = 1,
    NoMemory = 2,
    Implementation = 3,
}

pub(crate) struct Client<D> {
    socket: BufferedSocket,
    map: ObjectMap<Data<D>>,
    debug: bool,
    last_serial: u32,
    pub(crate) id: ClientId,
    pub(crate) killed: bool,
    pub(crate) data: Arc<dyn ClientData<D>>,
}

impl<D> Client<D> {
    fn next_serial(&mut self) -> u32 {
        self.last_serial = self.last_serial.wrapping_add(1);
        self.last_serial
    }
}

impl<D> Client<D> {
    pub(crate) fn new(
        stream: UnixStream,
        id: ClientId,
        debug: bool,
        data: Arc<dyn ClientData<D>>,
    ) -> Self {
        let socket = BufferedSocket::new(unsafe { Socket::from_raw_fd(stream.into_raw_fd()) });
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

        data.initialized(id);

        Client { socket, map, debug, id, killed: false, last_serial: 0, data }
    }

    pub(crate) fn create_object(
        &mut self,
        interface: &'static Interface,
        version: u32,
        user_data: Arc<dyn ObjectData<D>>,
    ) -> ObjectId {
        let serial = self.next_serial();
        let id = self.map.server_insert_new(Object {
            interface,
            version,
            data: Data { serial, user_data },
        });
        ObjectId { id, serial, client_id: self.id, interface }
    }

    pub(crate) fn object_info(&self, id: ObjectId) -> Result<ObjectInfo, InvalidId> {
        let object = self.get_object(id)?;
        Ok(ObjectInfo { id: id.id, interface: object.interface, version: object.version })
    }

    pub(crate) fn send_event(
        &mut self,
        Message { sender_id: object_id, opcode, args }: Message<ObjectId>,
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

        if !check_for_signature(message_desc.signature, &args) {
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
            crate::rs::debug::print_send_message(
                object.interface.name,
                object_id.id,
                message_desc.name,
                &args,
            );
        }

        let mut msg_args = SmallVec::with_capacity(args.len());
        let mut arg_interfaces = message_desc.arg_interfaces.iter();
        for (i, arg) in args.into_iter().enumerate() {
            msg_args.push(match arg {
                Argument::Array(a) => Argument::Array(a),
                Argument::Int(i) => Argument::Int(i),
                Argument::Uint(u) => Argument::Uint(u),
                Argument::Str(s) => Argument::Str(s),
                Argument::Fixed(f) => Argument::Fixed(f),
                Argument::Fd(f) => Argument::Fd(f),
                Argument::NewId(o) => {
                    if o.id != 0 {
                        if o.client_id != self.id {
                            panic!("Attempting to send an event with objects from wrong client.")
                        }
                        let object = self.get_object(o)?;
                        let child_interface = match message_desc.child_interface {
                            Some(iface) => iface,
                            None => panic!("Trying to send event {}@{}.{} which creates an object without specifying its interface, this is unsupported.", object_id.interface.name, object_id.id, message_desc.name),
                        };
                        if !same_interface(child_interface, object.interface) {
                            panic!("Event {}@{}.{} expects a newid argument of interface {} but {} was provided instead.", object.interface.name, object_id.id, message_desc.name, child_interface.name, object.interface.name);
                        }
                    } else {
                        if !matches!(message_desc.signature[i], ArgumentType::NewId(AllowNull::Yes)) {
                            panic!("Request {}@{}.{} expects an non-null newid argument.", object.interface.name, object_id.id, message_desc.name);
                        }
                    }
                    Argument::Object(o.id)
                },
                Argument::Object(o) => {
                    if o.id != 0 {
                        if o.client_id != self.id {
                            panic!("Attempting to send an event with objects from wrong client.")
                        }
                        let object = self.get_object(o)?;
                        let next_interface = arg_interfaces.next().unwrap();
                        if !same_interface_or_anonymous(next_interface, object.interface) {
                            panic!("Event {}@{}.{} expects an object argument of interface {} but {} was provided instead.", object.interface.name, object_id.id, message_desc.name, next_interface.name, object.interface.name);
                        }
                    } else {
                        if !matches!(message_desc.signature[i], ArgumentType::Object(AllowNull::Yes)) {
                            panic!("Request {}@{}.{} expects an non-null object argument.", object.interface.name, object_id.id, message_desc.name);
                        }
                    }
                    Argument::Object(o.id)
                }
            });
        }

        let msg = Message { sender_id: object_id.id, opcode, args: msg_args };

        if let Err(_) = self.socket.write_message(&msg) {
            self.kill(DisconnectReason::ConnectionClosed);
        }

        // Handle destruction if relevant
        if message_desc.is_destructor {
            self.map.remove(object_id.id);
            object.data.user_data.destroyed(self.id, object_id);
            self.send_delete_id(object_id);
        }

        Ok(())
    }

    pub(crate) fn send_delete_id(&mut self, object_id: ObjectId) {
        let msg = message!(1, 1, [Argument::Uint(object_id.id)]);
        if let Err(_) = self.socket.write_message(&msg) {
            self.kill(DisconnectReason::ConnectionClosed);
        }
    }

    pub(crate) fn get_object_data(
        &self,
        id: ObjectId,
    ) -> Result<Arc<dyn ObjectData<D>>, InvalidId> {
        let object = self.get_object(id)?;
        Ok(object.data.user_data)
    }

    pub(crate) fn post_display_error(&mut self, code: DisplayError, message: CString) {
        self.post_error(
            ObjectId { id: 1, interface: &WL_DISPLAY_INTERFACE, client_id: self.id, serial: 0 },
            code as u32,
            message,
        )
    }

    pub(crate) fn post_error(&mut self, object_id: ObjectId, error_code: u32, message: CString) {
        let converted_message = message.to_string_lossy().into();
        // errors are ignored, as the client will be killed anyway
        let _ = self.send_event(message!(
            ObjectId { id: 1, interface: &WL_DISPLAY_INTERFACE, client_id: self.id, serial: 0 },
            0, // wl_display.error
            [
                Argument::Object(object_id),
                Argument::Uint(error_code),
                Argument::Str(Box::new(message)),
            ],
        ));
        let _ = self.flush();
        self.kill(DisconnectReason::ProtocolError(ProtocolError {
            code: error_code,
            object_id: object_id.id,
            object_interface: object_id.interface.name.into(),
            message: converted_message,
        }));
    }

    pub(crate) fn kill(&mut self, reason: DisconnectReason) {
        self.killed = true;
        self.data.disconnected(self.id, reason);
    }

    pub(crate) fn flush(&mut self) -> std::io::Result<()> {
        self.socket.flush()
    }

    pub(crate) fn all_objects<'a>(&'a self) -> impl Iterator<Item = ObjectId> + 'a {
        let client_id = self.id;
        self.map.all_objects().map(move |(id, obj)| ObjectId {
            id,
            client_id,
            interface: obj.interface,
            serial: obj.data.serial,
        })
    }

    pub(crate) fn next_request(&mut self) -> std::io::Result<(Message<u32>, Object<Data<D>>)> {
        if self.killed {
            return Err(nix::errno::Errno::EPIPE.into());
        }
        loop {
            let map = &self.map;
            let msg = match self.socket.read_one_message(|id, opcode| {
                map.find(id)
                    .and_then(|o| o.interface.requests.get(opcode as usize))
                    .map(|desc| desc.signature)
            }) {
                Ok(msg) => msg,
                Err(MessageParseError::MissingData) | Err(MessageParseError::MissingFD) => {
                    // need to read more data
                    if let Err(e) = self.socket.fill_incoming_buffers() {
                        if e.kind() != std::io::ErrorKind::WouldBlock {
                            self.kill(DisconnectReason::ConnectionClosed);
                        }
                        return Err(e);
                    }
                    continue;
                }
                Err(MessageParseError::Malformed) => {
                    self.kill(DisconnectReason::ConnectionClosed);
                    return Err(nix::errno::Errno::EPROTO.into());
                }
            };

            let obj = self.map.find(msg.sender_id).unwrap();
            return Ok((msg, obj));
        }
    }

    fn get_object(&self, id: ObjectId) -> Result<Object<Data<D>>, InvalidId> {
        let object = self.map.find(id.id).ok_or(InvalidId)?;
        if object.data.serial != id.serial {
            return Err(InvalidId);
        }
        Ok(object)
    }

    pub(crate) fn handle_display_request(
        &mut self,
        message: Message<u32>,
        registry: &mut Registry<D>,
    ) {
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
                        self.post_display_error(
                            DisplayError::InvalidObject,
                            CString::new(format!("Invalid new_id: {}.", new_id)).unwrap(),
                        );
                        return;
                    }
                    let cb_id = ObjectId {
                        id: new_id,
                        client_id: self.id,
                        serial,
                        interface: &WL_CALLBACK_INTERFACE,
                    };
                    // send wl_callback.done(0)
                    self.send_event(message!(cb_id, 0, [Argument::Uint(0)])).unwrap();
                } else {
                    unreachable!()
                }
            }
            // wl_display.get_registry(new id wl_registry)
            1 => {
                if let &[Argument::NewId(new_id)] = &message.args[..] {
                    let serial = self.next_serial();
                    let registry_obj = Object {
                        interface: &WL_REGISTRY_INTERFACE,
                        version: 1,
                        data: Data { user_data: Arc::new(DumbObjectData), serial },
                    };
                    let registry_id = ObjectId {
                        id: new_id,
                        serial,
                        client_id: self.id,
                        interface: &WL_REGISTRY_INTERFACE,
                    };
                    if let Err(()) = self.map.insert_at(new_id, registry_obj) {
                        self.post_display_error(
                            DisplayError::InvalidObject,
                            CString::new(format!("Invalid new_id: {}.", new_id)).unwrap(),
                        );
                        return;
                    }
                    let _ = registry.send_all_globals_to(registry_id, self);
                } else {
                    unreachable!()
                }
            }
            _ => {
                // unkown opcode, kill the client
                self.post_display_error(
                    DisplayError::InvalidMethod,
                    CString::new(format!(
                        "Unknown opcode {} for interface wl_display.",
                        message.opcode
                    ))
                    .unwrap(),
                );
            }
        }
    }

    pub(crate) fn handle_registry_request(
        &mut self,
        message: Message<u32>,
        registry: &mut Registry<D>,
        data: &mut D,
    ) -> Option<(ClientId, GlobalId, ObjectId, Arc<dyn GlobalHandler<D>>)> {
        match message.opcode {
            // wl_registry.bind(uint name, str interface, uint version, new id)
            0 => {
                if let &[Argument::Uint(name), Argument::Str(ref interface_name), Argument::Uint(version), Argument::NewId(new_id)] =
                    &message.args[..]
                {
                    if let Some((interface, global_id, handler)) =
                        registry.check_bind(self.id, name, interface_name, version)
                    {
                        let user_data = handler
                            .clone()
                            .make_data(data, &ObjectInfo { id: new_id, interface, version });
                        let serial = self.next_serial();
                        let object =
                            Object { interface, version, data: Data { serial, user_data } };
                        if let Err(()) = self.map.insert_at(new_id, object) {
                            self.post_display_error(
                                DisplayError::InvalidObject,
                                CString::new(format!("Invalid new_id: {}.", new_id)).unwrap(),
                            );
                            return None;
                        }
                        Some((
                            self.id,
                            global_id,
                            ObjectId { id: new_id, client_id: self.id, interface, serial },
                            handler.clone(),
                        ))
                    } else {
                        self.post_display_error(
                            DisplayError::InvalidObject,
                            CString::new(format!(
                                "Invalid binding of {} version {} for global {}.",
                                interface_name.to_string_lossy(),
                                version,
                                name
                            ))
                            .unwrap(),
                        );
                        None
                    }
                } else {
                    unreachable!()
                }
            }
            _ => {
                // unkown opcode, kill the client
                self.post_display_error(
                    DisplayError::InvalidMethod,
                    CString::new(format!(
                        "Unknown opcode {} for interface wl_registry.",
                        message.opcode
                    ))
                    .unwrap(),
                );
                None
            }
        }
    }

    pub(crate) fn process_request(
        &mut self,
        object: &Object<Data<D>>,
        message: Message<u32>,
        data: &mut D,
    ) -> Option<(SmallVec<[Argument<ObjectId>; INLINE_ARGS]>, bool)> {
        let message_desc = object.interface.requests.get(message.opcode as usize).unwrap();
        // Convert the arguments and create the new object if applicable
        let mut new_args = SmallVec::with_capacity(message.args.len());
        let mut arg_interfaces = message_desc.arg_interfaces.iter();
        for (i, arg) in message.args.into_iter().enumerate() {
            new_args.push(match arg {
                Argument::Array(a) => Argument::Array(a),
                Argument::Int(i) => Argument::Int(i),
                Argument::Uint(u) => Argument::Uint(u),
                Argument::Str(s) => Argument::Str(s),
                Argument::Fixed(f) => Argument::Fixed(f),
                Argument::Fd(f) => Argument::Fd(f),
                Argument::Object(o) => {
                    if o != 0 {
                        // Lookup the object to make the appropriate Id
                        let obj = match self.map.find(o) {
                            Some(o) => o,
                            None => {
                                self.post_display_error(
                                    DisplayError::InvalidObject,
                                    CString::new(format!("Unknown id: {}.", o)).unwrap()
                                );
                                return None;
                            }
                        };
                        if let Some(next_interface) = arg_interfaces.next() {
                            if !same_interface_or_anonymous(next_interface, obj.interface) {
                                self.post_display_error(
                                    DisplayError::InvalidObject,
                                    CString::new(format!(
                                        "Invalid object {} in request {}.{}: expected {} but got {}.",
                                        o,
                                        object.interface.name,
                                        message_desc.name,
                                        next_interface.name,
                                        obj.interface.name,
                                    )).unwrap()
                                );
                                return None;
                            }
                        }
                        Argument::Object(ObjectId { id: o, client_id: self.id, serial: obj.data.serial, interface: obj.interface })
                    } else if matches!(message_desc.signature[i], ArgumentType::Object(AllowNull::Yes)) {
                        Argument::Object(ObjectId { id: 0, client_id: self.id, serial: 0, interface: &ANONYMOUS_INTERFACE })
                    } else {
                        self.post_display_error(
                            DisplayError::InvalidObject,
                            CString::new(format!(
                                "Invalid null object in request {}.{}.",
                                object.interface.name,
                                message_desc.name,
                            )).unwrap()
                        );
                        return None;
                    }
                }
                Argument::NewId(new_id) => {
                    // An object should be created
                    let child_interface = match message_desc.child_interface {
                        Some(iface) => iface,
                        None => panic!("Received request {}@{}.{} which creates an object without specifying its interface, this is unsupported.", object.interface.name, message.sender_id, message_desc.name),
                    };

                    let child_udata = object.data.user_data.clone().make_child(data, &ObjectInfo {
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

                    let child_id = ObjectId { id: new_id, client_id: self.id, serial: child_obj.data.serial, interface: child_obj.interface };

                    if let Err(()) = self.map.insert_at(new_id, child_obj) {
                        // abort parsing, this is an unrecoverable error
                        self.post_display_error(
                            DisplayError::InvalidObject,
                            CString::new(format!("Invalid new_id: {}.", new_id)).unwrap()
                        );
                        return None;
                    }

                    Argument::NewId(child_id)
                }
            });
        }

        Some((new_args, message_desc.is_destructor))
    }
}

struct DumbObjectData;

impl<D> ObjectData<D> for DumbObjectData {
    fn make_child(
        self: Arc<Self>,
        _data: &mut D,
        _child_info: &ObjectInfo,
    ) -> Arc<dyn ObjectData<D>> {
        unreachable!()
    }

    fn request(
        &self,
        _handle: &mut Handle<D>,
        _data: &mut D,
        _client_id: ClientId,
        _msg: Message<ObjectId>,
    ) {
        unreachable!()
    }

    fn destroyed(&self, _client_id: ClientId, _object_id: ObjectId) {}
}

pub(crate) struct ClientStore<D> {
    clients: Vec<Option<Client<D>>>,
    last_serial: u32,
    debug: bool,
}

impl<D> ClientStore<D> {
    pub(crate) fn new(debug: bool) -> Self {
        ClientStore { clients: Vec::new(), last_serial: 0, debug }
    }

    pub(crate) fn create_client(
        &mut self,
        stream: UnixStream,
        data: Arc<dyn ClientData<D>>,
    ) -> ClientId {
        let serial = self.next_serial();
        // Find the next free place
        let (id, place) = match self.clients.iter_mut().enumerate().find(|(_, c)| c.is_none()) {
            Some((id, place)) => (id, place),
            None => {
                self.clients.push(None);
                (self.clients.len() - 1, self.clients.last_mut().unwrap())
            }
        };

        let id = ClientId { id: id as u32, serial };

        *place = Some(Client::new(stream, id, self.debug, data));

        id
    }

    pub(crate) fn get_client(&self, id: ClientId) -> Result<&Client<D>, InvalidId> {
        match self.clients.get(id.id as usize) {
            Some(&Some(ref client)) if client.id == id => Ok(client),
            _ => Err(InvalidId),
        }
    }

    pub(crate) fn get_client_mut(&mut self, id: ClientId) -> Result<&mut Client<D>, InvalidId> {
        match self.clients.get_mut(id.id as usize) {
            Some(&mut Some(ref mut client)) if client.id == id => Ok(client),
            _ => Err(InvalidId),
        }
    }

    pub(crate) fn cleanup(&mut self) -> SmallVec<[ClientId; 1]> {
        let mut cleaned = SmallVec::new();
        for place in &mut self.clients {
            if place.as_ref().map(|client| client.killed).unwrap_or(false) {
                // Remove the client from the store and flush it one last time before fropping it
                let mut client = place.take().unwrap();
                let _ = client.flush();
                cleaned.push(client.id);
            }
        }
        cleaned
    }

    fn next_serial(&mut self) -> u32 {
        self.last_serial = self.last_serial.wrapping_add(1);
        self.last_serial
    }

    pub(crate) fn clients_mut<'a>(&'a mut self) -> impl Iterator<Item = &'a mut Client<D>> {
        self.clients.iter_mut().flat_map(|o| o.as_mut()).filter(|c| !c.killed)
    }

    pub(crate) fn all_clients_id<'a>(&'a self) -> impl Iterator<Item = ClientId> + 'a {
        self.clients
            .iter()
            .flat_map(|opt| opt.as_ref().filter(|c| !c.killed).map(|client| client.id))
    }
}
