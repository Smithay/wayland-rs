use std::{
    any::Any,
    ffi::CString,
    os::unix::io::OwnedFd,
    os::unix::net::UnixStream,
    sync::{Arc, Mutex, Weak},
};

use crate::{
    protocol::{ANONYMOUS_INTERFACE, Interface, Message, ObjectInfo, same_interface},
    rs::DEFAULT_MAX_BUFFER_SIZE,
    types::server::{DisconnectReason, GlobalInfo, InvalidId},
};

use super::{
    ClientData, ClientId, Credentials, GlobalHandler, GlobalId, InnerClientId, InnerGlobalId,
    InnerObjectId, ObjectData, ObjectId, client::ClientStore, registry::Registry,
};

pub(crate) type PendingDestructor<D> = (Arc<dyn ObjectData<D>>, InnerClientId, InnerObjectId);

#[derive(Debug)]
pub struct State<D: 'static> {
    pub(crate) clients: ClientStore<D>,
    pub(crate) registry: Registry<D>,
    pub(crate) pending_destructors: Vec<PendingDestructor<D>>,
    pub(crate) poll_fd: OwnedFd,
    pub(crate) default_max_buffer_size: usize,
}

impl<D> State<D> {
    pub(crate) fn new(poll_fd: OwnedFd) -> Self {
        let debug =
            matches!(std::env::var_os("WAYLAND_DEBUG"), Some(str) if str == "1" || str == "server");
        Self {
            clients: ClientStore::new(debug),
            registry: Registry::new(),
            pending_destructors: Vec::new(),
            poll_fd,
            default_max_buffer_size: DEFAULT_MAX_BUFFER_SIZE,
        }
    }

    pub(crate) fn cleanup<'a>(&mut self) -> impl FnOnce(&super::Handle, &mut D) + 'a + use<'a, D> {
        let dead_clients = self.clients.cleanup(&mut self.pending_destructors);
        self.registry.cleanup(&dead_clients, &self.pending_destructors);
        // return a closure that will do the cleanup once invoked
        let pending_destructors = std::mem::take(&mut self.pending_destructors);
        move |handle, data| {
            for (object_data, client_id, object_id) in pending_destructors {
                object_data.clone().destroyed(
                    handle,
                    data,
                    &ClientId { id: client_id },
                    &ObjectId { id: object_id },
                );
            }
            std::mem::drop(dead_clients);
        }
    }

    pub(crate) fn flush(&mut self, client: Option<&InnerClientId>) -> std::io::Result<()> {
        if let Some(client) = client {
            match self.clients.get_client_mut(*client) {
                Ok(client) => client.flush(),
                Err(InvalidId) => Ok(()),
            }
        } else {
            for client in self.clients.clients_mut() {
                let _ = client.flush();
            }
            Ok(())
        }
    }

    pub fn set_default_max_buffer_size(&mut self, max_buffer_size: usize) {
        self.default_max_buffer_size = max_buffer_size;
    }

    fn set_client_max_buffer_size(&mut self, client: &InnerClientId, max_buffer_size: usize) {
        if let Ok(client) = self.clients.get_client_mut(*client) {
            client.socket.set_max_buffer_size(Some(max_buffer_size));
        }
    }
}

impl<D> super::ErasedHandle for State<D> {
    fn object_info(&self, id: &InnerObjectId) -> Result<ObjectInfo, InvalidId> {
        self.clients.get_client(id.client_id)?.object_info(id)
    }

    fn insert_client(
        &mut self,
        stream: UnixStream,
        data: Arc<dyn ClientData>,
    ) -> std::io::Result<InnerClientId> {
        let id = self.clients.create_client(stream, data, self.default_max_buffer_size);
        let client = self.clients.get_client(id).unwrap();

        // register the client to the internal epoll
        #[cfg(any(target_os = "linux", target_os = "android", target_os = "redox"))]
        let ret = {
            use rustix::event::epoll;
            epoll::add(
                &self.poll_fd,
                client,
                epoll::EventData::new_u64(id.as_u64()),
                epoll::EventFlags::IN,
            )
        };

        #[cfg(any(
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd",
            target_os = "macos"
        ))]
        let ret = {
            use rustix::event::kqueue::*;
            use std::os::unix::io::{AsFd, AsRawFd};

            let evt = Event::new(
                EventFilter::Read(client.as_fd().as_raw_fd()),
                EventFlags::ADD | EventFlags::RECEIPT,
                id.as_u64() as *mut _,
            );

            let events: &mut [Event] = &mut [];
            unsafe { kevent(&self.poll_fd, &[evt], events, None).map(|_| ()) }
        };

        match ret {
            Ok(()) => Ok(id),
            Err(e) => {
                self.kill_client(&id, DisconnectReason::ConnectionClosed);
                Err(e.into())
            }
        }
    }

    fn get_client(&self, id: &InnerObjectId) -> Result<InnerClientId, InvalidId> {
        if self.clients.get_client(id.client_id).is_ok() {
            Ok(id.client_id)
        } else {
            Err(InvalidId)
        }
    }

    fn get_client_data(&self, id: &InnerClientId) -> Result<Arc<dyn ClientData>, InvalidId> {
        let client = self.clients.get_client(*id)?;
        Ok(client.data.clone())
    }

    fn get_client_credentials(&self, id: &InnerClientId) -> Result<Credentials, InvalidId> {
        let client = self.clients.get_client(*id)?;
        Ok(client.get_credentials())
    }

    fn with_all_clients(&self, f: &mut dyn FnMut(&ClientId)) {
        for client in self.clients.all_clients_id() {
            f(&client)
        }
    }

    fn with_all_objects_for(
        &self,
        client_id: &InnerClientId,
        f: &mut dyn FnMut(&ObjectId),
    ) -> Result<(), InvalidId> {
        let client = self.clients.get_client(*client_id)?;
        for object in client.all_objects() {
            f(&object)
        }
        Ok(())
    }

    fn object_for_protocol_id(
        &self,
        client_id: &InnerClientId,
        interface: &'static Interface,
        protocol_id: u32,
    ) -> Result<InnerObjectId, InvalidId> {
        let client = self.clients.get_client(*client_id)?;
        let object = client.object_for_protocol_id(protocol_id)?;
        if same_interface(interface, object.interface) { Ok(object) } else { Err(InvalidId) }
    }

    fn get_object_data_any(
        &self,
        id: &InnerObjectId,
    ) -> Result<Arc<dyn Any + Send + Sync>, InvalidId> {
        self.clients
            .get_client(id.client_id)?
            .get_object_data(id)
            .map(|arc| -> Arc<dyn Any + Send + Sync> { arc })
    }

    fn send_event(&mut self, msg: Message<ObjectId>) -> Result<(), InvalidId> {
        self.clients
            .get_client_mut(msg.sender_id.id.client_id)?
            .send_event(msg, Some(&mut self.pending_destructors))
    }

    fn post_error(&mut self, object_id: &InnerObjectId, error_code: u32, message: CString) {
        if let Ok(client) = self.clients.get_client_mut(object_id.client_id) {
            client.post_error(object_id, error_code, message)
        }
    }

    fn kill_client(&mut self, client_id: &InnerClientId, reason: DisconnectReason) {
        if let Ok(client) = self.clients.get_client_mut(*client_id) {
            client.kill(reason)
        }
    }
    fn global_info(&self, id: &InnerGlobalId) -> Result<GlobalInfo, InvalidId> {
        self.registry.get_info(*id)
    }

    fn global_name(&self, global_id: &InnerGlobalId, client_id: &InnerClientId) -> Option<u32> {
        let client = self.clients.get_client(*client_id).ok()?;
        let handler = self.registry.get_handler(*global_id).ok()?;
        let name = global_id.id;

        let can_view = handler.can_view(
            &ClientId { id: *client_id },
            &client.data,
            &GlobalId { id: *global_id },
        );

        if can_view { Some(name) } else { None }
    }

    fn flush(&mut self, client: Option<&InnerClientId>) -> std::io::Result<()> {
        self.flush(client)
    }

    fn set_default_max_buffer_size(&mut self, max_buffer_size: usize) {
        self.set_default_max_buffer_size(max_buffer_size)
    }

    fn set_client_max_buffer_size(&mut self, client: &InnerClientId, max_buffer_size: usize) {
        self.set_client_max_buffer_size(client, max_buffer_size)
    }
}
