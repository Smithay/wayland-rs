use std::{
    ffi::CString,
    sync::{Arc, Mutex, Weak},
};

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
pub struct State<D: 'static> {
    pub(crate) clients: ClientStore<D>,
    pub(crate) registry: Registry<D>,
    pub(crate) pending_destructors: Vec<PendingDestructor<D>>,
}

impl<D> State<D> {
    pub(crate) fn new() -> Self {
        let debug =
            matches!(std::env::var_os("WAYLAND_DEBUG"), Some(str) if str == "1" || str == "server");
        State {
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

#[derive(Clone)]
pub struct InnerHandle {
    pub(crate) state: Arc<Mutex<dyn ErasedState + Send>>,
}

impl std::fmt::Debug for InnerHandle {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt.debug_struct("InnerHandle[rs]").finish_non_exhaustive()
    }
}

#[derive(Clone)]
pub struct WeakInnerHandle {
    pub(crate) state: Weak<Mutex<dyn ErasedState + Send>>,
}

impl std::fmt::Debug for WeakInnerHandle {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt.debug_struct("WeakInnerHandle[rs]").finish_non_exhaustive()
    }
}

impl WeakInnerHandle {
    pub fn upgrade(&self) -> Option<InnerHandle> {
        self.state.upgrade().map(|state| InnerHandle { state })
    }
}

impl InnerHandle {
    pub fn downgrade(&self) -> WeakInnerHandle {
        WeakInnerHandle { state: Arc::downgrade(&self.state) }
    }

    pub fn object_info(&self, id: InnerObjectId) -> Result<ObjectInfo, InvalidId> {
        self.state.lock().unwrap().object_info(id)
    }

    pub fn get_client(&self, id: InnerObjectId) -> Result<ClientId, InvalidId> {
        self.state.lock().unwrap().get_client(id)
    }

    pub fn get_client_data<D: 'static>(
        &self,
        id: InnerClientId,
    ) -> Result<Arc<dyn ClientData<D>>, InvalidId> {
        let mut state = self.state.lock().unwrap();
        let state = (&mut *state as &mut dyn ErasedState)
            .downcast_mut::<State<D>>()
            .expect("Wrong type parameter passed to Handle::get_client_data().");
        let client = state.clients.get_client(id)?;
        Ok(client.data.clone())
    }

    pub fn get_client_data_any(
        &self,
        id: InnerClientId,
    ) -> Result<Arc<dyn std::any::Any + Send + Sync>, InvalidId> {
        self.state.lock().unwrap().get_client_data_any(id)
    }

    pub fn get_client_credentials(&self, id: InnerClientId) -> Result<Credentials, InvalidId> {
        self.state.lock().unwrap().get_client_credentials(id)
    }

    pub fn with_all_clients(&self, mut f: impl FnMut(ClientId)) {
        self.state.lock().unwrap().with_all_clients(&mut f)
    }

    pub fn with_all_objects_for(
        &self,
        client_id: InnerClientId,
        mut f: impl FnMut(ObjectId),
    ) -> Result<(), InvalidId> {
        self.state.lock().unwrap().with_all_objects_for(client_id, &mut f)
    }

    pub fn object_for_protocol_id(
        &self,
        client_id: InnerClientId,
        interface: &'static Interface,
        protocol_id: u32,
    ) -> Result<ObjectId, InvalidId> {
        self.state.lock().unwrap().object_for_protocol_id(client_id, interface, protocol_id)
    }

    pub fn create_object<D: 'static>(
        &self,
        client_id: InnerClientId,
        interface: &'static Interface,
        version: u32,
        data: Arc<dyn ObjectData<D>>,
    ) -> Result<ObjectId, InvalidId> {
        let mut state = self.state.lock().unwrap();
        let state = (&mut *state as &mut dyn ErasedState)
            .downcast_mut::<State<D>>()
            .expect("Wrong type parameter passed to Handle::create_object().");
        let client = state.clients.get_client_mut(client_id)?;
        Ok(ObjectId { id: client.create_object(interface, version, data) })
    }

    pub fn null_id() -> ObjectId {
        ObjectId {
            id: InnerObjectId {
                id: 0,
                serial: 0,
                client_id: InnerClientId { id: 0, serial: 0 },
                interface: &ANONYMOUS_INTERFACE,
            },
        }
    }

    pub fn send_event(&self, msg: Message<ObjectId>) -> Result<(), InvalidId> {
        self.state.lock().unwrap().send_event(msg)
    }

    pub fn get_object_data<D: 'static>(
        &self,
        id: InnerObjectId,
    ) -> Result<Arc<dyn ObjectData<D>>, InvalidId> {
        let mut state = self.state.lock().unwrap();
        let state = (&mut *state as &mut dyn ErasedState)
            .downcast_mut::<State<D>>()
            .expect("Wrong type parameter passed to Handle::get_object_data().");
        state.clients.get_client(id.client_id.clone())?.get_object_data(id)
    }

    pub fn get_object_data_any(
        &self,
        id: InnerObjectId,
    ) -> Result<Arc<dyn std::any::Any + Send + Sync>, InvalidId> {
        self.state.lock().unwrap().get_object_data_any(id)
    }

    pub fn set_object_data<D: 'static>(
        &self,
        id: InnerObjectId,
        data: Arc<dyn ObjectData<D>>,
    ) -> Result<(), InvalidId> {
        let mut state = self.state.lock().unwrap();
        let state = (&mut *state as &mut dyn ErasedState)
            .downcast_mut::<State<D>>()
            .expect("Wrong type parameter passed to Handle::set_object_data().");
        state.clients.get_client_mut(id.client_id.clone())?.set_object_data(id, data)
    }

    pub fn post_error(&self, object_id: InnerObjectId, error_code: u32, message: CString) {
        self.state.lock().unwrap().post_error(object_id, error_code, message)
    }

    pub fn kill_client(&self, client_id: InnerClientId, reason: DisconnectReason) {
        self.state.lock().unwrap().kill_client(client_id, reason)
    }

    pub fn create_global<D: 'static>(
        &self,
        interface: &'static Interface,
        version: u32,
        handler: Arc<dyn GlobalHandler<D>>,
    ) -> InnerGlobalId {
        let mut state = self.state.lock().unwrap();
        let state = (&mut *state as &mut dyn ErasedState)
            .downcast_mut::<State<D>>()
            .expect("Wrong type parameter passed to Handle::create_global().");
        state.registry.create_global(interface, version, handler, &mut state.clients)
    }

    pub fn disable_global(&self, id: InnerGlobalId) {
        self.state.lock().unwrap().disable_global(id)
    }

    pub fn remove_global(&self, id: InnerGlobalId) {
        self.state.lock().unwrap().remove_global(id)
    }

    pub fn global_info(&self, id: InnerGlobalId) -> Result<GlobalInfo, InvalidId> {
        self.state.lock().unwrap().global_info(id)
    }

    pub fn get_global_handler<D: 'static>(
        &self,
        id: InnerGlobalId,
    ) -> Result<Arc<dyn GlobalHandler<D>>, InvalidId> {
        let mut state = self.state.lock().unwrap();
        let state = (&mut *state as &mut dyn ErasedState)
            .downcast_mut::<State<D>>()
            .expect("Wrong type parameter passed to Handle::get_global_handler().");
        state.registry.get_handler(id)
    }
}

pub(crate) trait ErasedState: downcast_rs::Downcast {
    fn object_info(&self, id: InnerObjectId) -> Result<ObjectInfo, InvalidId>;
    fn get_client(&self, id: InnerObjectId) -> Result<ClientId, InvalidId>;
    fn get_client_data_any(
        &self,
        id: InnerClientId,
    ) -> Result<Arc<dyn std::any::Any + Send + Sync>, InvalidId>;
    fn get_client_credentials(&self, id: InnerClientId) -> Result<Credentials, InvalidId>;
    fn with_all_clients(&self, f: &mut dyn FnMut(ClientId));
    fn with_all_objects_for(
        &self,
        client_id: InnerClientId,
        f: &mut dyn FnMut(ObjectId),
    ) -> Result<(), InvalidId>;
    fn object_for_protocol_id(
        &self,
        client_id: InnerClientId,
        interface: &'static Interface,
        protocol_id: u32,
    ) -> Result<ObjectId, InvalidId>;
    fn get_object_data_any(
        &self,
        id: InnerObjectId,
    ) -> Result<Arc<dyn std::any::Any + Send + Sync>, InvalidId>;
    fn send_event(&mut self, msg: Message<ObjectId>) -> Result<(), InvalidId>;
    fn post_error(&mut self, object_id: InnerObjectId, error_code: u32, message: CString);
    fn kill_client(&mut self, client_id: InnerClientId, reason: DisconnectReason);
    fn disable_global(&mut self, id: InnerGlobalId);
    fn remove_global(&mut self, id: InnerGlobalId);
    fn global_info(&self, id: InnerGlobalId) -> Result<GlobalInfo, InvalidId>;
}

downcast_rs::impl_downcast!(ErasedState);

impl<D> ErasedState for State<D> {
    fn object_info(&self, id: InnerObjectId) -> Result<ObjectInfo, InvalidId> {
        self.clients.get_client(id.client_id.clone())?.object_info(id)
    }

    fn get_client(&self, id: InnerObjectId) -> Result<ClientId, InvalidId> {
        if self.clients.get_client(id.client_id.clone()).is_ok() {
            Ok(ClientId { id: id.client_id })
        } else {
            Err(InvalidId)
        }
    }

    fn get_client_data_any(
        &self,
        id: InnerClientId,
    ) -> Result<Arc<dyn std::any::Any + Send + Sync>, InvalidId> {
        let client = self.clients.get_client(id)?;
        Ok(client.data.clone().into_any_arc())
    }

    fn get_client_credentials(&self, id: InnerClientId) -> Result<Credentials, InvalidId> {
        let client = self.clients.get_client(id)?;
        Ok(client.get_credentials())
    }

    fn with_all_clients(&self, f: &mut dyn FnMut(ClientId)) {
        for client in self.clients.all_clients_id() {
            f(client)
        }
    }

    fn with_all_objects_for(
        &self,
        client_id: InnerClientId,
        f: &mut dyn FnMut(ObjectId),
    ) -> Result<(), InvalidId> {
        let client = self.clients.get_client(client_id)?;
        for object in client.all_objects() {
            f(object)
        }
        Ok(())
    }

    fn object_for_protocol_id(
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

    fn get_object_data_any(
        &self,
        id: InnerObjectId,
    ) -> Result<Arc<dyn std::any::Any + Send + Sync>, InvalidId> {
        self.clients
            .get_client(id.client_id.clone())?
            .get_object_data(id)
            .map(|arc| arc.into_any_arc())
    }

    fn send_event(&mut self, msg: Message<ObjectId>) -> Result<(), InvalidId> {
        self.clients
            .get_client_mut(msg.sender_id.id.client_id.clone())?
            .send_event(msg, Some(&mut self.pending_destructors))
    }

    fn post_error(&mut self, object_id: InnerObjectId, error_code: u32, message: CString) {
        if let Ok(client) = self.clients.get_client_mut(object_id.client_id.clone()) {
            client.post_error(object_id, error_code, message)
        }
    }

    fn kill_client(&mut self, client_id: InnerClientId, reason: DisconnectReason) {
        if let Ok(client) = self.clients.get_client_mut(client_id) {
            client.kill(reason)
        }
    }

    fn disable_global(&mut self, id: InnerGlobalId) {
        self.registry.disable_global(id, &mut self.clients)
    }

    fn remove_global(&mut self, id: InnerGlobalId) {
        self.registry.remove_global(id, &mut self.clients)
    }

    fn global_info(&self, id: InnerGlobalId) -> Result<GlobalInfo, InvalidId> {
        self.registry.get_info(id)
    }
}
