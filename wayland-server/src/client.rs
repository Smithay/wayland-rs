use std::sync::Arc;

use wayland_backend::{
    protocol::ProtocolError,
    server::{ClientId, DisconnectReason, InvalidId, ObjectData},
};

use crate::{dispatch::ResourceData, Dispatch, DisplayHandle, Resource};

#[derive(Debug)]
pub struct Client {
    pub(crate) id: ClientId,
    pub(crate) data: Arc<dyn std::any::Any + Send + Sync>,
}

impl Client {
    pub(crate) fn from_id(handle: &DisplayHandle, id: ClientId) -> Result<Client, InvalidId> {
        let data = handle.handle.get_client_data_any(id.clone())?;
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
        handle: &DisplayHandle,
    ) -> Result<crate::backend::Credentials, InvalidId> {
        handle.handle.get_client_credentials(self.id.clone())
    }

    pub fn create_resource<
        I: Resource + 'static,
        U: Send + Sync + 'static,
        D: Dispatch<I, U> + 'static,
    >(
        &self,
        handle: &DisplayHandle,
        version: u32,
        user_data: U,
    ) -> Result<I, InvalidId> {
        let id = handle.handle.create_object::<D>(
            self.id.clone(),
            I::interface(),
            version,
            Arc::new(ResourceData::<I, U>::new(user_data)) as Arc<_>,
        )?;
        I::from_id(handle, id)
    }

    pub fn create_resource_from_objdata<I: Resource + 'static, D: 'static>(
        &self,
        handle: &DisplayHandle,
        version: u32,
        obj_data: Arc<dyn ObjectData<D>>,
    ) -> Result<I, InvalidId> {
        let id =
            handle.handle.create_object::<D>(self.id.clone(), I::interface(), version, obj_data)?;
        I::from_id(handle, id)
    }

    pub fn object_from_protocol_id<I: Resource + 'static>(
        &self,
        handle: &DisplayHandle,
        protocol_id: u32,
    ) -> Result<I, InvalidId> {
        let object_id =
            handle.handle.object_for_protocol_id(self.id.clone(), I::interface(), protocol_id)?;
        I::from_id(handle, object_id)
    }

    pub fn kill(&self, handle: &DisplayHandle, error: ProtocolError) {
        handle.handle.kill_client(self.id.clone(), DisconnectReason::ProtocolError(error))
    }
}

impl PartialEq for Client {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
