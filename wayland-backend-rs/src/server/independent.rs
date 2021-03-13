use std::{os::unix::net::UnixStream, sync::Arc};

use wayland_commons::{
    server::{ClientData, IndependentBackend, ServerBackend},
    Never,
};

use super::{ClientId, GlobalId, Handle, ObjectId};

pub struct IndependentServerBackend<D> {
    handle: Handle<D, IndependentServerBackend<D>>,
}

impl<D> ServerBackend<D> for IndependentServerBackend<D> {
    type ObjectId = ObjectId;
    type ClientId = ClientId;
    type GlobalId = GlobalId;
    type Handle = Handle<D, IndependentServerBackend<D>>;
    type InitError = Never;

    fn new() -> Result<Self, Never> {
        Ok(IndependentServerBackend { handle: Handle::new() })
    }

    fn insert_client(
        &mut self,
        stream: UnixStream,
        data: Arc<dyn ClientData<D, Self>>,
    ) -> std::io::Result<Self::ClientId> {
        Ok(self.handle.clients.create_client(stream, data))
    }

    fn flush(&mut self, client: Option<Self::ClientId>) -> std::io::Result<()> {
        self.handle.flush(client)
    }

    fn handle(&mut self) -> &mut Self::Handle {
        &mut self.handle
    }
}

impl<D> IndependentBackend<D> for IndependentServerBackend<D> {
    fn dispatch_events_for(&mut self, data: &mut D, client_id: ClientId) -> std::io::Result<usize> {
        let ret = self.handle.dispatch_events_for(data, client_id);
        self.handle.cleanup();
        ret
    }
}
