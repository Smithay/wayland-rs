use std::{os::unix::net::UnixStream, sync::Arc};

use wayland_commons::{
    server::{ClientData, IndependentBackend, ServerBackend},
    Never,
};

use super::{ClientId, GlobalId, Handle, ObjectId};

pub struct IndependentServerBackend {
    handle: Handle<IndependentServerBackend>,
}

impl ServerBackend for IndependentServerBackend {
    type ObjectId = ObjectId;
    type ClientId = ClientId;
    type GlobalId = GlobalId;
    type Handle = Handle<IndependentServerBackend>;
    type InitError = Never;

    fn new() -> Result<Self, Never> {
        Ok(IndependentServerBackend { handle: Handle::new() })
    }

    fn insert_client(
        &mut self,
        stream: UnixStream,
        data: Arc<dyn ClientData<Self>>,
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

impl IndependentBackend for IndependentServerBackend {
    fn dispatch_events_for(&mut self, client_id: ClientId) -> std::io::Result<usize> {
        self.handle.dispatch_events_for(client_id)
    }
}
