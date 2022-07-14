use std::{os::unix::net::UnixStream, sync::Arc};

use wayland_backend::{
    protocol::ObjectInfo,
    server::{Backend, ClientData, GlobalId, Handle, InitError, InvalidId, ObjectId},
};

use crate::{
    global::{GlobalData, GlobalDispatch},
    Client, Resource,
};

#[derive(Debug)]
pub struct Display<D: 'static> {
    backend: Backend<D>,
}

impl<D: 'static> Display<D> {
    pub fn new() -> Result<Display<D>, InitError> {
        Ok(Display { backend: Backend::new()? })
    }

    pub fn handle(&self) -> DisplayHandle {
        DisplayHandle { handle: self.backend.handle() }
    }
    pub fn dispatch_clients(&mut self, data: &mut D) -> std::io::Result<usize> {
        self.backend.dispatch_all_clients(data)
    }

    pub fn flush_clients(&mut self) -> std::io::Result<()> {
        self.backend.flush(None)
    }

    pub fn backend(&mut self) -> &mut Backend<D> {
        &mut self.backend
    }
}

/// A handle to the Wayland display
///
/// A display handle may be constructed from a [`Handle`] using it's [`From`] implementation.
#[derive(Clone)]
pub struct DisplayHandle {
    pub(crate) handle: Handle,
}

impl std::fmt::Debug for DisplayHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DisplayHandle").finish_non_exhaustive()
    }
}

impl DisplayHandle {
    /// Returns the underlying [`Handle`] from `wayland-backend`.
    pub fn backend_handle(&self) -> Handle {
        self.handle.clone()
    }

    pub fn get_object_data(
        &self,
        id: ObjectId,
    ) -> Result<Arc<dyn std::any::Any + Send + Sync + 'static>, InvalidId> {
        self.handle.get_object_data_any(id)
    }

    pub fn object_info(&self, id: ObjectId) -> Result<ObjectInfo, InvalidId> {
        self.handle.object_info(id)
    }

    pub fn insert_client(
        &mut self,
        stream: UnixStream,
        data: Arc<dyn ClientData>,
    ) -> std::io::Result<Client> {
        let id = self.handle.insert_client(stream, data.clone())?;
        Ok(Client { id, data })
    }

    pub fn get_client(&self, id: ObjectId) -> Result<Client, InvalidId> {
        let client_id = self.handle.get_client(id)?;
        Client::from_id(self, client_id)
    }

    pub fn send_event<I: Resource>(&self, resource: &I, event: I::Event) -> Result<(), InvalidId> {
        let msg = resource.write_event(self, event)?;
        self.handle.send_event(msg)
    }

    pub fn post_error<I: Resource>(&self, resource: &I, code: u32, error: String) {
        self.handle.post_error(resource.id(), code, std::ffi::CString::new(error).unwrap())
    }

    pub fn create_global<D, I: Resource + 'static, U: Send + Sync + 'static>(
        &self,
        version: u32,
        data: U,
    ) -> GlobalId
    where
        D: GlobalDispatch<I, U> + 'static,
    {
        self.handle.create_global::<D>(
            I::interface(),
            version,
            Arc::new(GlobalData { data, _types: std::marker::PhantomData }),
        )
    }

    pub fn disable_global<D: 'static>(&self, id: GlobalId) {
        self.handle.disable_global::<D>(id)
    }

    pub fn remove_global<D: 'static>(&self, id: GlobalId) {
        self.handle.remove_global::<D>(id)
    }
}

impl From<Handle> for DisplayHandle {
    /// Creates a [`DisplayHandle`] using a [`Handle`](Handle) from `wayland-backend`.
    fn from(handle: Handle) -> Self {
        Self { handle }
    }
}
