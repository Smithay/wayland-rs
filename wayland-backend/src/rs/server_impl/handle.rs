use std::{ffi::CString, sync::Arc};

use crate::{
    protocol::{same_interface, Interface, Message, ObjectInfo, ANONYMOUS_INTERFACE},
    types::server::{DisconnectReason, GlobalInfo, InvalidId},
};

use super::{
    client::ClientStore, registry::Registry, ClientData, ClientId, Credentials, GlobalHandler,
    InnerClientId, InnerGlobalId, InnerObjectId, ObjectData, ObjectId,
};

pub(crate) type PendingDestructor<D> = (Arc<dyn ObjectData<D>>, InnerClientId, InnerObjectId);

#[derive(Debug)]
pub struct InnerHandle<D> {
    pub(crate) clients: ClientStore<D>,
    pub(crate) registry: Registry<D>,
    pub(crate) pending_destructors: Vec<PendingDestructor<D>>,
}

impl<D> InnerHandle<D> {
    pub(crate) fn new() -> Self {
        let debug =
            matches!(std::env::var_os("WAYLAND_DEBUG"), Some(str) if str == "1" || str == "server");
        InnerHandle {
            clients: ClientStore::new(debug),
            registry: Registry::new(),
            pending_destructors: Vec::new(),
        }
    }

    pub(crate) fn cleanup(&mut self, data: &mut D) {
        let dead_clients = self.clients.cleanup(data);
        self.registry.cleanup(&dead_clients);
        // invoke all pending destructors if relevant
        for (object_data, client_id, object_id) in self.pending_destructors.drain(..) {
            object_data.destroyed(data, ClientId { id: client_id }, ObjectId { id: object_id });
        }
    }

    pub(crate) fn flush(&mut self, client: Option<ClientId>) -> std::io::Result<()> {
        if let Some(ClientId { id: client }) = client {
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

impl<D> InnerHandle<D> {
    pub fn object_info(&self, id: InnerObjectId) -> Result<ObjectInfo, InvalidId> {
        self.clients.get_client(id.client_id.clone())?.object_info(id)
    }

    pub fn get_client(&self, id: InnerObjectId) -> Result<ClientId, InvalidId> {
        if self.clients.get_client(id.client_id.clone()).is_ok() {
            Ok(ClientId { id: id.client_id })
        } else {
            Err(InvalidId)
        }
    }

    pub fn get_client_data(&self, id: InnerClientId) -> Result<Arc<dyn ClientData<D>>, InvalidId> {
        let client = self.clients.get_client(id)?;
        Ok(client.data.clone())
    }

    pub fn get_client_credentials(&self, id: InnerClientId) -> Result<Credentials, InvalidId> {
        let client = self.clients.get_client(id)?;
        Ok(client.get_credentials())
    }

    pub fn all_clients<'a>(&'a self) -> Box<dyn Iterator<Item = ClientId> + 'a> {
        Box::new(self.clients.all_clients_id())
    }

    pub fn all_objects_for<'a>(
        &'a self,
        client_id: InnerClientId,
    ) -> Result<Box<dyn Iterator<Item = ObjectId> + 'a>, InvalidId> {
        let client = self.clients.get_client(client_id)?;
        Ok(Box::new(client.all_objects()))
    }

    pub fn object_for_protocol_id(
        &self,
        client_id: InnerClientId,
        interface: &'static Interface,
        protocol_id: u32,
    ) -> Result<ObjectId, InvalidId> {
        let client = self.clients.get_client(client_id)?;
        let object = client.object_for_protocol_id(protocol_id)?;
        if same_interface(interface, object.interface) {
            Ok(ObjectId { id: object })
        } else {
            Err(InvalidId)
        }
    }

    pub fn create_object(
        &mut self,
        client_id: InnerClientId,
        interface: &'static Interface,
        version: u32,
        data: Arc<dyn ObjectData<D>>,
    ) -> Result<ObjectId, InvalidId> {
        let client = self.clients.get_client_mut(client_id)?;
        Ok(ObjectId { id: client.create_object(interface, version, data) })
    }

    pub fn null_id(&mut self) -> ObjectId {
        ObjectId {
            id: InnerObjectId {
                id: 0,
                serial: 0,
                client_id: InnerClientId { id: 0, serial: 0 },
                interface: &ANONYMOUS_INTERFACE,
            },
        }
    }

    pub fn send_event(&mut self, msg: Message<ObjectId>) -> Result<(), InvalidId> {
        self.clients
            .get_client_mut(msg.sender_id.id.client_id.clone())?
            .send_event(msg, Some(&mut self.pending_destructors))
    }

    pub fn get_object_data(&self, id: InnerObjectId) -> Result<Arc<dyn ObjectData<D>>, InvalidId> {
        self.clients.get_client(id.client_id.clone())?.get_object_data(id)
    }

    pub fn set_object_data(
        &mut self,
        id: InnerObjectId,
        data: Arc<dyn ObjectData<D>>,
    ) -> Result<(), InvalidId> {
        self.clients.get_client_mut(id.client_id.clone())?.set_object_data(id, data)
    }

    pub fn post_error(&mut self, object_id: InnerObjectId, error_code: u32, message: CString) {
        if let Ok(client) = self.clients.get_client_mut(object_id.client_id.clone()) {
            client.post_error(object_id, error_code, message)
        }
    }

    pub fn kill_client(&mut self, client_id: InnerClientId, reason: DisconnectReason) {
        if let Ok(client) = self.clients.get_client_mut(client_id) {
            client.kill(reason)
        }
    }

    pub fn create_global(
        &mut self,
        interface: &'static Interface,
        version: u32,
        handler: Arc<dyn GlobalHandler<D>>,
    ) -> InnerGlobalId {
        self.registry.create_global(interface, version, handler, &mut self.clients)
    }

    pub fn disable_global(&mut self, id: InnerGlobalId) {
        self.registry.disable_global(id, &mut self.clients)
    }

    pub fn remove_global(&mut self, id: InnerGlobalId) {
        self.registry.remove_global(id, &mut self.clients)
    }

    pub fn global_info(&self, id: InnerGlobalId) -> Result<GlobalInfo, InvalidId> {
        self.registry.get_info(id)
    }

    pub fn get_global_handler(
        &self,
        id: InnerGlobalId,
    ) -> Result<Arc<dyn GlobalHandler<D>>, InvalidId> {
        self.registry.get_handler(id)
    }
}
