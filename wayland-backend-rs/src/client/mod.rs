use std::{os::unix::io::RawFd, sync::Arc};

use wayland_commons::backend_api::{
    client::{
        BackendHandle, ClientBackend, ConnectError, InvalidId, ObjectData, ProtocolError,
        WaylandError,
    },
    Argument, Interface, MessageDesc, ObjectInfo,
};

#[derive(Debug, Clone)]
pub struct Id {}

pub struct Handle {}

pub struct Backend {
    handle: Handle,
}

impl ClientBackend for Backend {
    type ObjectId = Id;
    type Handle = Handle;

    fn connect(fd: RawFd) -> Result<Self, ConnectError> {
        todo!()
    }

    fn connection_fd(&self) -> RawFd {
        todo!()
    }

    fn dispatch_events(&mut self) -> std::io::Result<usize> {
        todo!()
    }

    fn handle(&self) -> &Self::Handle {
        &self.handle
    }
}

impl BackendHandle<Backend> for Handle {
    fn display_id(&self) -> Id {
        todo!()
    }

    fn last_error(&self) -> Option<WaylandError> {
        todo!()
    }

    fn info(&self, id: Id) -> Result<ObjectInfo, InvalidId> {
        todo!()
    }

    fn send_request(&self, id: Id, opcode: u16, args: &[Argument<Id>]) -> Result<(), InvalidId> {
        todo!()
    }

    fn placeholder_id(&self, spec: Option<(&'static Interface, u32)>) -> Id {
        todo!()
    }

    fn send_constructor(
        &self,
        id: Id,
        opcode: u16,
        args: &[Argument<Id>],
        data: Option<Arc<dyn ObjectData<Backend>>>,
    ) -> Result<Id, InvalidId> {
        todo!()
    }

    fn get_data(&self, id: Id) -> Result<Arc<dyn ObjectData<Backend>>, InvalidId> {
        todo!()
    }
}
