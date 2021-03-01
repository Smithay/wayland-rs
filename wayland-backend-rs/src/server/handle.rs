use std::sync::Arc;

use wayland_commons::{
    core_interfaces::{ANONYMOUS_INTERFACE, WL_DISPLAY_INTERFACE},
    server::{
        BackendHandle, ClientData, CommonPollBackend, DisconnectReason, GlobalHandler, GlobalInfo,
        IndependentBackend, InvalidId, NoWaylandLib, ObjectData, ServerBackend,
    },
    Argument, ArgumentType, Interface, ObjectInfo, ProtocolError,
};

use super::{client::Client, ClientId, GlobalId, ObjectId};

pub struct Handle<B> {
    clients: Vec<Client<B>>,
    last_serial: u32,
}

impl<B> Handle<B> {
    fn next_serial(&mut self) -> u32 {
        self.last_serial = self.last_serial.wrapping_add(1);
        self.last_serial
    }
}

impl<B> BackendHandle<B> for Handle<B>
where
    B: ServerBackend<ClientId = ClientId, ObjectId = ObjectId, GlobalId = GlobalId>,
{
    fn object_info(&self, id: B::ObjectId) -> Result<ObjectInfo, InvalidId> {
        todo!()
    }

    fn get_client(&self, id: B::ObjectId) -> Result<ClientId, InvalidId> {
        todo!()
    }

    fn get_client_data(&self, id: B::ClientId) -> Result<Arc<dyn ClientData<B>>, InvalidId> {
        todo!()
    }

    fn all_clients<'a>(&'a self) -> Box<dyn Iterator<Item = B::ClientId> + 'a> {
        todo!()
    }

    fn all_objects_for<'a>(
        &'a self,
        client_id: B::ClientId,
    ) -> Box<dyn Iterator<Item = B::ObjectId> + 'a> {
        todo!()
    }

    fn create_object(
        &mut self,
        client: ClientId,
        interface: &'static Interface,
        version: u32,
        data: Arc<dyn ObjectData<B>>,
    ) -> ObjectId {
        todo!()
    }

    fn send_event(
        &mut self,
        object_id: ObjectId,
        opcode: u16,
        args: &[Argument<ObjectId>],
    ) -> Result<(), InvalidId> {
        todo!()
    }

    fn get_object_data(&self, id: ObjectId) -> Result<Arc<dyn ObjectData<B>>, InvalidId> {
        todo!()
    }

    fn post_error(&mut self, object_id: ObjectId, error_code: u32, message: String) {
        todo!()
    }

    fn kill_client(&mut self, client_id: ClientId) {
        todo!()
    }

    fn create_global(
        &mut self,
        interface: &'static Interface,
        version: u32,
        handler: Arc<dyn GlobalHandler<B>>,
    ) -> GlobalId {
        todo!()
    }

    fn disable_global(&mut self, id: GlobalId) {
        todo!()
    }

    fn remove_global(&mut self, id: GlobalId) {
        todo!()
    }

    fn global_info(&self, id: B::GlobalId) -> Result<GlobalInfo, InvalidId> {
        todo!()
    }

    fn get_global_handler(&self, id: B::GlobalId) -> Result<Arc<dyn GlobalHandler<B>>, InvalidId> {
        todo!()
    }
}

pub struct ClientIterator;

impl Iterator for ClientIterator {
    type Item = ClientId;
    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }
}
