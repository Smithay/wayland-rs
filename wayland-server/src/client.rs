use std::sync::Arc;

use wayland_backend::{
    protocol::ProtocolError,
    server::{ClientId, DisconnectReason, InvalidId},
};

use crate::{dispatch::ResourceData, Dispatch, DisplayHandle, Resource};

pub struct Client {
    pub(crate) id: ClientId,
    pub(crate) data: Arc<dyn std::any::Any + Send + Sync>,
}

impl Client {
    pub(crate) fn from_id<D>(
        handle: &mut DisplayHandle<'_, D>,
        id: ClientId,
    ) -> Result<Client, InvalidId> {
        let data = handle.inner.handle().get_client_data(id.clone())?.into_any_arc();
        Ok(Client { id, data })
    }

    pub fn id(&self) -> ClientId {
        self.id.clone()
    }

    pub fn get_data<Data: 'static>(&self) -> Option<&Data> {
        (&*self.data).downcast_ref()
    }

    pub fn create_resource<I: Resource + 'static, D: Dispatch<I> + 'static>(
        &self,
        handle: &mut DisplayHandle<'_, D>,
        version: u32,
    ) -> Result<I, InvalidId> {
        let id = handle.inner.handle().create_object(
            self.id.clone(),
            I::interface(),
            version,
            Arc::new(ResourceData::<I, <D as Dispatch<I>>::UserData>::default()),
        )?;
        I::from_id(handle, id)
    }

    pub fn kill<D>(&self, handle: &mut DisplayHandle<'_, D>, error: ProtocolError) {
        handle.inner.handle().kill_client(self.id.clone(), DisconnectReason::ProtocolError(error))
    }
}
