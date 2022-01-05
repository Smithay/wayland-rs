use std::{
    os::unix::net::UnixStream,
    sync::{Arc, Mutex, MutexGuard},
};

use wayland_backend::{
    protocol::{Interface, Message, ObjectInfo},
    server::{
        Backend, ClientData, ClientId, Credentials, DisconnectReason, GlobalId, Handle, InitError,
        InvalidId, ObjectId,
    },
};

use crate::{
    global::{GlobalData, GlobalDispatch},
    Client, Resource,
};

#[derive(Debug, Clone)]
pub struct Display<D> {
    backend: Arc<Mutex<Backend<D>>>,
}

impl<D: 'static> Display<D> {
    pub fn new() -> Result<Display<D>, InitError> {
        Ok(Display { backend: Arc::new(Mutex::new(Backend::new()?)) })
    }

    pub fn handle(&self) -> DisplayHandle<'_> {
        DisplayHandle {
            inner: HandleInner::Guard(
                (&*self.backend as &Mutex<dyn ErasedDisplayHandle>).lock().unwrap(),
            ),
        }
    }

    pub fn insert_client(
        &self,
        stream: UnixStream,
        data: Arc<dyn ClientData<D>>,
    ) -> std::io::Result<Client> {
        let id = self.backend.lock().unwrap().insert_client(stream, data.clone())?;
        Ok(Client { id, data: data.into_any_arc() })
    }

    pub fn dispatch_clients(&self, data: &mut D) -> std::io::Result<usize> {
        self.backend.lock().unwrap().dispatch_all_clients(data)
    }

    pub fn flush_clients(&self) -> std::io::Result<()> {
        self.backend.lock().unwrap().flush(None)
    }

    pub fn create_global<I: Resource + 'static>(
        &self,
        version: u32,
        data: <D as GlobalDispatch<I>>::GlobalData,
    ) -> GlobalId
    where
        D: GlobalDispatch<I> + 'static,
    {
        self.backend.lock().unwrap().handle().create_global(
            I::interface(),
            version,
            Arc::new(GlobalData { data }),
        )
    }

    pub fn disable_global(&self, id: GlobalId) {
        self.backend.lock().unwrap().handle().disable_global(id)
    }

    pub fn remove_global(&self, id: GlobalId) {
        self.backend.lock().unwrap().handle().remove_global(id)
    }
}

pub struct DisplayHandle<'a> {
    pub(crate) inner: HandleInner<'a>,
}

impl<'a> std::fmt::Debug for DisplayHandle<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DisplayHandle").finish_non_exhaustive()
    }
}

pub(crate) enum HandleInner<'a> {
    Handle(&'a mut dyn ErasedDisplayHandle),
    Guard(MutexGuard<'a, dyn ErasedDisplayHandle>),
}

impl<'a> HandleInner<'a> {
    #[inline]
    pub(crate) fn handle(&mut self) -> &mut dyn ErasedDisplayHandle {
        match self {
            HandleInner::Handle(handle) => *handle,
            HandleInner::Guard(ref mut guard) => &mut **guard,
        }
    }

    pub(crate) fn typed_handle<D: 'static>(&mut self) -> Option<&mut Handle<D>> {
        match self {
            HandleInner::Handle(handle) => handle.downcast_mut(),
            HandleInner::Guard(ref mut guard) => {
                (&mut **guard).downcast_mut::<Backend<D>>().map(|backend| backend.handle())
            }
        }
    }
}

impl<'a> DisplayHandle<'a> {
    pub(crate) fn from_handle<D: 'static>(handle: &'a mut Handle<D>) -> DisplayHandle<'a> {
        DisplayHandle { inner: HandleInner::Handle(handle) }
    }

    pub fn get_object_data(
        &mut self,
        id: ObjectId,
    ) -> Result<Arc<dyn std::any::Any + Send + Sync + 'static>, InvalidId> {
        self.inner.handle().get_object_data(id)
    }

    pub fn object_info(&mut self, id: ObjectId) -> Result<ObjectInfo, InvalidId> {
        self.inner.handle().object_info(id)
    }

    pub fn get_client(&mut self, id: ObjectId) -> Result<Client, InvalidId> {
        self.inner.handle().get_client(id)
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
}

/* Dynamic dispatch plumbing for erasing type parameter on DisplayHandle */
pub(crate) trait ErasedDisplayHandle: downcast_rs::Downcast {
    fn get_object_data(
        &mut self,
        id: ObjectId,
    ) -> Result<Arc<dyn std::any::Any + Send + Sync + 'static>, InvalidId>;
    fn object_info(&mut self, id: ObjectId) -> Result<ObjectInfo, InvalidId>;
    fn get_client(&mut self, id: ObjectId) -> Result<Client, InvalidId>;
    fn null_id(&mut self) -> ObjectId;
    fn send_event(&mut self, msg: Message<ObjectId>) -> Result<(), InvalidId>;
    fn object_for_protocol_id(
        &mut self,
        cid: ClientId,
        interface: &'static Interface,
        pid: u32,
    ) -> Result<ObjectId, InvalidId>;
    fn post_error(&mut self, id: ObjectId, code: u32, msg: std::ffi::CString);
    fn get_client_credentials(&mut self, id: ClientId) -> Result<Credentials, InvalidId>;
    fn get_client_data(
        &mut self,
        id: ClientId,
    ) -> Result<Arc<dyn std::any::Any + Send + Sync>, InvalidId>;
    fn kill_client(&mut self, id: ClientId, reason: DisconnectReason);
}

downcast_rs::impl_downcast!(ErasedDisplayHandle);

impl<D: 'static> ErasedDisplayHandle for Handle<D> {
    fn get_object_data(
        &mut self,
        id: ObjectId,
    ) -> Result<Arc<dyn std::any::Any + Send + Sync + 'static>, InvalidId> {
        Handle::<D>::get_object_data(self, id).map(|udata| udata.into_any_arc())
    }

    fn object_info(&mut self, id: ObjectId) -> Result<ObjectInfo, InvalidId> {
        Handle::<D>::object_info(self, id)
    }

    fn get_client(&mut self, id: ObjectId) -> Result<Client, InvalidId> {
        let client_id = Handle::<D>::get_client(self, id)?;
        Client::from_id(&mut DisplayHandle::from_handle(self), client_id)
    }

    fn null_id(&mut self) -> ObjectId {
        Handle::<D>::null_id(self)
    }

    fn send_event(&mut self, msg: Message<ObjectId>) -> Result<(), InvalidId> {
        Handle::<D>::send_event(self, msg)
    }

    fn object_for_protocol_id(
        &mut self,
        cid: ClientId,
        interface: &'static Interface,
        pid: u32,
    ) -> Result<ObjectId, InvalidId> {
        Handle::<D>::object_for_protocol_id(self, cid, interface, pid)
    }

    fn post_error(&mut self, id: ObjectId, code: u32, msg: std::ffi::CString) {
        Handle::<D>::post_error(self, id, code, msg)
    }

    fn get_client_credentials(&mut self, id: ClientId) -> Result<Credentials, InvalidId> {
        Handle::<D>::get_client_credentials(self, id)
    }

    fn get_client_data(
        &mut self,
        id: ClientId,
    ) -> Result<Arc<dyn std::any::Any + Send + Sync>, InvalidId> {
        Handle::<D>::get_client_data(self, id).map(|udata| udata.into_any_arc())
    }

    fn kill_client(&mut self, id: ClientId, reason: DisconnectReason) {
        Handle::<D>::kill_client(self, id, reason)
    }
}

impl<D: 'static> ErasedDisplayHandle for Backend<D> {
    fn get_object_data(
        &mut self,
        id: ObjectId,
    ) -> Result<Arc<dyn std::any::Any + Send + Sync + 'static>, InvalidId> {
        Handle::<D>::get_object_data(self.handle(), id).map(|udata| udata.into_any_arc())
    }

    fn object_info(&mut self, id: ObjectId) -> Result<ObjectInfo, InvalidId> {
        Handle::<D>::object_info(self.handle(), id)
    }

    fn get_client(&mut self, id: ObjectId) -> Result<Client, InvalidId> {
        let client_id = Handle::<D>::get_client(self.handle(), id)?;
        Client::from_id(&mut DisplayHandle::from_handle(self.handle()), client_id)
    }

    fn null_id(&mut self) -> ObjectId {
        Handle::<D>::null_id(self.handle())
    }

    fn send_event(&mut self, msg: Message<ObjectId>) -> Result<(), InvalidId> {
        Handle::<D>::send_event(self.handle(), msg)
    }

    fn object_for_protocol_id(
        &mut self,
        cid: ClientId,
        interface: &'static Interface,
        pid: u32,
    ) -> Result<ObjectId, InvalidId> {
        Handle::<D>::object_for_protocol_id(self.handle(), cid, interface, pid)
    }

    fn post_error(&mut self, id: ObjectId, code: u32, msg: std::ffi::CString) {
        Handle::<D>::post_error(self.handle(), id, code, msg)
    }

    fn get_client_credentials(&mut self, id: ClientId) -> Result<Credentials, InvalidId> {
        Handle::<D>::get_client_credentials(self.handle(), id)
    }

    fn get_client_data(
        &mut self,
        id: ClientId,
    ) -> Result<Arc<dyn std::any::Any + Send + Sync>, InvalidId> {
        Handle::<D>::get_client_data(self.handle(), id).map(|udata| udata.into_any_arc())
    }

    fn kill_client(&mut self, id: ClientId, reason: DisconnectReason) {
        Handle::<D>::kill_client(self.handle(), id, reason)
    }
}
