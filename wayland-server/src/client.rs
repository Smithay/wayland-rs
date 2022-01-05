use std::sync::Arc;

use wayland_backend::{
    protocol::ProtocolError,
    server::{ClientId, DisconnectReason, InvalidId},
};

use crate::{dispatch::ResourceData, Dispatch, DisplayHandle, Resource};

#[derive(Debug)]
pub struct Client {
    pub(crate) id: ClientId,
    pub(crate) data: Arc<dyn std::any::Any + Send + Sync>,
}

impl Client {
    pub(crate) fn from_id(
        handle: &mut DisplayHandle<'_>,
        id: ClientId,
    ) -> Result<Client, InvalidId> {
        let data = handle.inner.handle().get_client_data(id.clone())?;
        Ok(Client { id, data })
    }

    pub fn id(&self) -> ClientId {
        self.id.clone()
    }

    pub fn get_data<Data: 'static>(&self) -> Option<&Data> {
        (&*self.data).downcast_ref()
    }

    pub fn get_credentials(
        &self,
        handle: &mut DisplayHandle<'_>,
    ) -> Result<crate::backend::Credentials, InvalidId> {
        handle.inner.handle().get_client_credentials(self.id.clone())
    }

    pub fn create_resource<I: Resource + 'static, D: Dispatch<I> + 'static>(
        &self,
        handle: &mut DisplayHandle<'_>,
        version: u32,
        user_data: <D as Dispatch<I>>::UserData,
    ) -> Result<I, InvalidId> {
        let id = handle
            .inner
            .typed_handle::<D>()
            .expect("Wrong D type passed to Client::create_ressource")
            .create_object(
                self.id.clone(),
                I::interface(),
                version,
                Arc::new(ResourceData::<I, _>::new(user_data)),
            )?;
        I::from_id(handle, id)
    }

    pub fn object_from_protocol_id<I: Resource + 'static>(
        &self,
        handle: &mut DisplayHandle<'_>,
        protocol_id: u32,
    ) -> Result<I, InvalidId> {
        let object_id = handle.inner.handle().object_for_protocol_id(
            self.id.clone(),
            I::interface(),
            protocol_id,
        )?;
        I::from_id(handle, object_id)
    }

    pub fn kill(&self, handle: &mut DisplayHandle<'_>, error: ProtocolError) {
        handle.inner.handle().kill_client(self.id.clone(), DisconnectReason::ProtocolError(error))
    }
}

impl PartialEq for Client {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
