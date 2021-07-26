use std::{ffi::CString, sync::Arc};

use crate::{
    core_interfaces::{WL_DISPLAY_INTERFACE, WL_REGISTRY_INTERFACE},
    protocol::{same_interface, Argument, Interface, Message, ObjectInfo, ANONYMOUS_INTERFACE},
    types::server::{DisconnectReason, GlobalInfo, InvalidId},
};
use smallvec::SmallVec;

use super::{
    client::ClientStore, registry::Registry, ClientData, ClientId, Data, GlobalHandler, GlobalId,
    ObjectData, ObjectId,
};
use crate::rs::map::Object;

pub struct Handle<D> {
    pub(crate) clients: ClientStore<D>,
    pub(crate) registry: Registry<D>,
}

enum DispatchAction<D> {
    Request {
        object: Object<Data<D>>,
        object_id: ObjectId,
        opcode: u16,
        arguments: SmallVec<[Argument<ObjectId>; 4]>,
        is_destructor: bool,
    },
    Bind {
        object: ObjectId,
        client: ClientId,
        global: GlobalId,
        handler: Arc<dyn GlobalHandler<D>>,
    },
}

impl<D> Handle<D> {
    pub(crate) fn new() -> Self {
        let debug = match std::env::var_os("WAYLAND_DEBUG") {
            Some(str) if str == "1" || str == "server" => true,
            _ => false,
        };
        Handle { clients: ClientStore::new(debug), registry: Registry::new() }
    }

    pub(crate) fn cleanup(&mut self) {
        let dead_clients = self.clients.cleanup();
        self.registry.cleanup(&dead_clients);
    }

    pub(crate) fn dispatch_events_for(
        &mut self,
        data: &mut D,
        client_id: ClientId,
    ) -> std::io::Result<usize> {
        let mut dispatched = 0;
        loop {
            let action = if let Ok(client) = self.clients.get_client_mut(client_id) {
                let (message, object) = match client.next_request() {
                    Ok(v) => v,
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        if dispatched > 0 {
                            break;
                        } else {
                            return Err(e);
                        }
                    }
                    Err(e) => return Err(e),
                };
                dispatched += 1;
                if same_interface(object.interface, &WL_DISPLAY_INTERFACE) {
                    client.handle_display_request(message, &mut self.registry);
                    continue;
                } else if same_interface(object.interface, &WL_REGISTRY_INTERFACE) {
                    if let Some((client, global, object, handler)) =
                        client.handle_registry_request(message, &mut self.registry, data)
                    {
                        DispatchAction::Bind { client, global, object, handler }
                    } else {
                        continue;
                    }
                } else {
                    let object_id = ObjectId {
                        id: message.sender_id,
                        serial: object.data.serial,
                        interface: object.interface,
                        client_id: client.id,
                    };
                    let opcode = message.opcode;
                    let (arguments, is_destructor) =
                        match client.process_request(&object, message, data) {
                            Some(args) => args,
                            None => continue,
                        };
                    // Return the whole set to invoke the callback while handle is not borrower via client
                    DispatchAction::Request { object, object_id, opcode, arguments, is_destructor }
                }
            } else {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Invalid client ID",
                ));
            };
            match action {
                DispatchAction::Request { object, object_id, opcode, arguments, is_destructor } => {
                    object.data.user_data.request(
                        self,
                        data,
                        client_id,
                        Message { sender_id: object_id, opcode, args: arguments },
                    );
                    if is_destructor {
                        object.data.user_data.destroyed(client_id, object_id);
                        if let Ok(client) = self.clients.get_client_mut(client_id) {
                            client.send_delete_id(object_id);
                        }
                    }
                }
                DispatchAction::Bind { object, client, global, handler } => {
                    handler.bind(self, data, client, global, object);
                }
            }
        }
        Ok(dispatched)
    }

    pub(crate) fn flush(&mut self, client: Option<ClientId>) -> std::io::Result<()> {
        if let Some(client) = client {
            match self.clients.get_client_mut(client) {
                Ok(client) => client.flush(),
                Err(InvalidId) => Ok(()),
            }
        } else {
            for client in self.clients.clients_mut() {
                let _ = client.flush();
            }
            Ok(())
        }
    }
}

impl<D> Handle<D> {
    pub fn object_info(&self, id: ObjectId) -> Result<ObjectInfo, InvalidId> {
        self.clients.get_client(id.client_id)?.object_info(id)
    }

    pub fn get_client(&self, id: ObjectId) -> Result<ClientId, InvalidId> {
        if self.clients.get_client(id.client_id).is_ok() {
            Ok(id.client_id)
        } else {
            Err(InvalidId)
        }
    }

    pub fn get_client_data(&self, id: ClientId) -> Result<Arc<dyn ClientData<D>>, InvalidId> {
        let client = self.clients.get_client(id)?;
        Ok(client.data.clone())
    }

    pub fn all_clients<'a>(&'a self) -> Box<dyn Iterator<Item = ClientId> + 'a> {
        Box::new(self.clients.all_clients_id())
    }

    pub fn all_objects_for<'a>(
        &'a self,
        client_id: ClientId,
    ) -> Result<Box<dyn Iterator<Item = ObjectId> + 'a>, InvalidId> {
        let client = self.clients.get_client(client_id)?;
        Ok(Box::new(client.all_objects()))
    }

    pub fn create_object(
        &mut self,
        client_id: ClientId,
        interface: &'static Interface,
        version: u32,
        data: Arc<dyn ObjectData<D>>,
    ) -> Result<ObjectId, InvalidId> {
        let client = self.clients.get_client_mut(client_id)?;
        Ok(client.create_object(interface, version, data))
    }

    pub fn null_id(&mut self) -> ObjectId {
        ObjectId {
            id: 0,
            serial: 0,
            client_id: ClientId { id: 0, serial: 0 },
            interface: &ANONYMOUS_INTERFACE,
        }
    }

    pub fn send_event(&mut self, msg: Message<ObjectId>) -> Result<(), InvalidId> {
        self.clients.get_client_mut(msg.sender_id.client_id)?.send_event(msg)
    }

    pub fn get_object_data(&self, id: ObjectId) -> Result<Arc<dyn ObjectData<D>>, InvalidId> {
        self.clients.get_client(id.client_id)?.get_object_data(id)
    }

    pub fn post_error(&mut self, object_id: ObjectId, error_code: u32, message: CString) {
        if let Ok(client) = self.clients.get_client_mut(object_id.client_id) {
            client.post_error(object_id, error_code, message)
        }
    }

    pub fn kill_client(&mut self, client_id: ClientId, reason: DisconnectReason) {
        if let Ok(client) = self.clients.get_client_mut(client_id) {
            client.kill(reason)
        }
    }

    pub fn create_global(
        &mut self,
        interface: &'static Interface,
        version: u32,
        handler: Arc<dyn GlobalHandler<D>>,
    ) -> GlobalId {
        self.registry.create_global(interface, version, handler, &mut self.clients)
    }

    pub fn disable_global(&mut self, id: GlobalId) {
        self.registry.disable_global(id, &mut self.clients)
    }

    pub fn remove_global(&mut self, id: GlobalId) {
        self.registry.remove_global(id, &mut self.clients)
    }

    pub fn global_info(&self, id: GlobalId) -> Result<GlobalInfo, InvalidId> {
        self.registry.get_info(id)
    }

    pub fn get_global_handler(&self, id: GlobalId) -> Result<Arc<dyn GlobalHandler<D>>, InvalidId> {
        self.registry.get_handler(id)
    }
}
