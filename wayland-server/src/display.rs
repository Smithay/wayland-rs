use std::{
    os::unix::net::UnixStream,
    sync::{Arc, Mutex, MutexGuard},
};

use wayland_backend::{
    protocol::ObjectInfo,
    server::{Backend, ClientData, GlobalId, Handle, InitError, InvalidId, ObjectData, ObjectId},
};

use crate::{
    global::{GlobalData, GlobalDispatch},
    Client, Resource,
};

#[derive(Debug, Clone)]
pub struct Display<D> {
    backend: Arc<Mutex<Backend<D>>>,
}

impl<D> Display<D> {
    pub fn new() -> Result<Display<D>, InitError> {
        Ok(Display { backend: Arc::new(Mutex::new(Backend::new()?)) })
    }

    pub fn handle(&self) -> DisplayHandle<'_, D> {
        DisplayHandle { inner: HandleInner::Guard(self.backend.lock().unwrap()) }
    }

    pub fn insert_client(
        &mut self,
        stream: UnixStream,
        data: Arc<dyn ClientData<D>>,
    ) -> std::io::Result<Client> {
        let id = self.backend.lock().unwrap().insert_client(stream, data.clone())?;
        Ok(Client { id, data: data.into_any_arc() })
    }

    pub fn dispatch_clients(&mut self, data: &mut D) -> std::io::Result<usize> {
        self.backend.lock().unwrap().dispatch_all_clients(data)
    }

    pub fn flush_clients(&mut self) -> std::io::Result<()> {
        self.backend.lock().unwrap().flush(None)
    }
}

#[derive(Debug)]
pub struct DisplayHandle<'a, D> {
    pub(crate) inner: HandleInner<'a, D>,
}

#[derive(Debug)]
pub(crate) enum HandleInner<'a, D> {
    Handle(&'a mut Handle<D>),
    Guard(MutexGuard<'a, Backend<D>>),
}

impl<'a, D> HandleInner<'a, D> {
    #[inline]
    pub(crate) fn handle(&mut self) -> &mut Handle<D> {
        match self {
            HandleInner::Handle(handle) => handle,
            HandleInner::Guard(guard) => guard.handle(),
        }
    }
}

impl<'a, D> DisplayHandle<'a, D> {
    pub(crate) fn from_handle(handle: &'a mut Handle<D>) -> DisplayHandle<'a, D> {
        DisplayHandle { inner: HandleInner::Handle(handle) }
    }

    pub fn get_object_data(&mut self, id: ObjectId) -> Result<Arc<dyn ObjectData<D>>, InvalidId> {
        self.inner.handle().get_object_data(id)
    }

    pub fn object_info(&mut self, id: ObjectId) -> Result<ObjectInfo, InvalidId> {
        self.inner.handle().object_info(id)
    }

    pub fn get_client(&mut self, id: ObjectId) -> Result<Client, InvalidId> {
        let client_id = self.inner.handle().get_client(id)?;
        Client::from_id(self, client_id)
    }

    pub fn null_id(&mut self) -> ObjectId {
        self.inner.handle().null_id()
    }

    pub fn send_event<I: Resource>(
        &mut self,
        resource: &I,
        event: I::Event,
    ) -> Result<(), InvalidId> {
        let msg = resource.write_event(self, event)?;
        self.inner.handle().send_event(msg)
    }

    pub fn post_error<I: Resource>(&mut self, resource: &I, code: u32, error: String) {
        self.inner.handle().post_error(resource.id(), code, std::ffi::CString::new(error).unwrap())
    }

    pub fn create_global<I: Resource + 'static>(
        &mut self,
        version: u32,
        data: <D as GlobalDispatch<I>>::GlobalData,
    ) -> GlobalId
    where
        D: GlobalDispatch<I> + 'static,
    {
        self.inner.handle().create_global(I::interface(), version, Arc::new(GlobalData { data }))
    }

    pub fn disable_global(&mut self, id: GlobalId) {
        self.inner.handle().disable_global(id)
    }

    pub fn remove_global(&mut self, id: GlobalId) {
        self.inner.handle().remove_global(id)
    }
}
