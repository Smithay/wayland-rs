use std::{
    os::unix::{io::RawFd, net::UnixStream},
    sync::Arc,
};

use wayland_commons::{
    client::{BackendHandle, ClientBackend, InvalidId, NoWaylandLib, ObjectData, WaylandError},
    Argument, Interface, MessageDesc, ObjectInfo, ProtocolError,
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

    unsafe fn connect(stream: UnixStream) -> Result<Self, NoWaylandLib> {
        todo!()
    }

    fn connection_fd(&self) -> RawFd {
        todo!()
    }

    fn flush(&mut self) -> std::io::Result<()> {
        todo!()
    }

    fn dispatch_events(&mut self) -> std::io::Result<usize> {
        todo!()
    }

    fn handle(&mut self) -> &mut Self::Handle {
        &mut self.handle
    }
}

impl BackendHandle<Backend> for Handle {
    fn display_id(&self) -> Id {
        todo!()
    }

    fn last_error(&self) -> Option<&WaylandError> {
        todo!()
    }

    fn info(&self, id: Id) -> Result<ObjectInfo, InvalidId> {
        todo!()
    }

    fn send_request(
        &mut self,
        id: Id,
        opcode: u16,
        args: &[Argument<Id>],
    ) -> Result<(), InvalidId> {
        todo!()
    }

    fn placeholder_id(&mut self, spec: Option<(&'static Interface, u32)>) -> Id {
        todo!()
    }

    fn send_constructor(
        &mut self,
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
