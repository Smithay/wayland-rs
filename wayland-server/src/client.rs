use std::sync::Arc;

use wayland_backend::{
    protocol::ProtocolError,
    server::{ClientData, ClientId, DisconnectReason, InvalidId, ObjectData},
};

use crate::{dispatch::ResourceData, Dispatch, DisplayHandle, Resource};

/// A struct representing a Wayland client connected to your compositor.
#[derive(Clone, Debug)]
pub struct Client {
    pub(crate) id: ClientId,
    pub(crate) data: Arc<dyn ClientData>,
}

impl Client {
    pub(crate) fn from_id(handle: &DisplayHandle, id: ClientId) -> Result<Self, InvalidId> {
        let data = handle.handle.get_client_data(id.clone())?;
        Ok(Self { id, data })
    }

    /// The backend [`ClientId`] of this client
    pub fn id(&self) -> ClientId {
        self.id.clone()
    }

    /// Access the data associated to this client
    ///
    /// Returns [`None`] if the provided `Data` type parameter is not the correct one.
    pub fn get_data<Data: ClientData + 'static>(&self) -> Option<&Data> {
        (*self.data).downcast_ref()
    }

    /// Access the pid/uid/gid of this client
    ///
    /// **Note:** You should be careful if you plan tu use this for security purposes, as it is possible for
    /// programs to spoof this kind of information.
    ///
    /// For a discussion about the subject of securely identifying clients, see
    /// <https://gitlab.freedesktop.org/wayland/weston/-/issues/206>
    pub fn get_credentials(
        &self,
        handle: &DisplayHandle,
    ) -> Result<crate::backend::Credentials, InvalidId> {
        handle.handle.get_client_credentials(self.id.clone())
    }

    /// Create a new Wayland object in the protocol state of this client
    ///
    /// The newly created resource should be immediately sent to the client through an associated event with
    /// a `new_id` argument. Not doing so risks corrupting the protocol state and causing protocol errors at
    /// a later time.
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

    /// Create a new Wayland object in the protocol state of this client, from an [`ObjectData`]
    ///
    /// This is a lower-level method than [`create_resource()`][Self::create_resource()], in case you need to
    /// bypass the [`Dispatch`] machinery.
    ///
    /// The newly created resource should be immediately sent to the client through an associated event with
    /// a `new_id` argument. Not doing so risks corrupting the protocol state and causing protocol errors at
    /// a later time.
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

    /// Attempt to retrieve an object from this client's protocol state from its protocol id
    ///
    /// Will fail if either the provided protocol id does not correspond to any object, or if the
    /// corresponding object is not of the interface `I`.
    pub fn object_from_protocol_id<I: Resource + 'static>(
        &self,
        handle: &DisplayHandle,
        protocol_id: u32,
    ) -> Result<I, InvalidId> {
        let object_id =
            handle.handle.object_for_protocol_id(self.id.clone(), I::interface(), protocol_id)?;
        I::from_id(handle, object_id)
    }

    /// Kill this client by triggering a protocol error
    pub fn kill(&self, handle: &DisplayHandle, error: ProtocolError) {
        handle.handle.kill_client(self.id.clone(), DisconnectReason::ProtocolError(error))
    }
}

impl PartialEq for Client {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
