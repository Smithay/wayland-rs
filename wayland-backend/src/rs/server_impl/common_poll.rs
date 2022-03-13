use std::{
    os::unix::{
        io::{AsRawFd, RawFd},
        net::UnixStream,
    },
    sync::Arc,
};

use super::{
    ClientData, Data, GlobalHandler, GlobalId, Handle, InnerClientId, InnerGlobalId, InnerObjectId,
    ObjectId,
};
use crate::{
    core_interfaces::{WL_DISPLAY_INTERFACE, WL_REGISTRY_INTERFACE},
    protocol::{same_interface, Argument, Message},
    rs::map::Object,
    types::server::{DisconnectReason, InitError},
};

#[cfg(target_os = "linux")]
use nix::sys::epoll::*;

#[cfg(any(
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
use nix::sys::event::*;
use smallvec::SmallVec;

use super::{ClientId, InnerHandle};

#[derive(Debug)]
pub struct InnerBackend<D> {
    handle: Handle<D>,
    poll_fd: RawFd,
}

impl<D> InnerBackend<D> {
    pub fn new() -> Result<Self, InitError> {
        #[cfg(target_os = "linux")]
        let poll_fd = epoll_create1(EpollCreateFlags::EPOLL_CLOEXEC)
            .map_err(Into::into)
            .map_err(InitError::Io)?;

        #[cfg(any(
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd"
        ))]
        let poll_fd = kqueue().map_err(Into::into).map_err(InitError::Io)?;

        Ok(InnerBackend { handle: Handle { handle: InnerHandle::new() }, poll_fd })
    }

    pub fn insert_client(
        &mut self,
        stream: UnixStream,
        data: Arc<dyn ClientData<D>>,
    ) -> std::io::Result<InnerClientId> {
        let client_fd = stream.as_raw_fd();
        let id = self.handle.handle.clients.create_client(stream, data);

        // register the client to the internal epoll
        #[cfg(target_os = "linux")]
        let ret = {
            let mut evt = EpollEvent::new(EpollFlags::EPOLLIN, id.as_u64());
            epoll_ctl(self.poll_fd, EpollOp::EpollCtlAdd, client_fd, &mut evt)
        };

        #[cfg(any(
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd"
        ))]
        let ret = {
            let evt = KEvent::new(
                client_fd as usize,
                EventFilter::EVFILT_READ,
                EventFlag::EV_ADD | EventFlag::EV_RECEIPT,
                FilterFlag::empty(),
                0,
                id.as_u64() as isize,
            );

            kevent_ts(self.poll_fd, &[evt], &mut [], None).map(|_| ())
        };

        match ret {
            Ok(()) => Ok(id),
            Err(e) => {
                self.handle.handle.kill_client(id, DisconnectReason::ConnectionClosed);
                Err(e.into())
            }
        }
    }

    pub fn flush(&mut self, client: Option<ClientId>) -> std::io::Result<()> {
        self.handle.handle.flush(client)
    }

    pub fn handle(&mut self) -> &mut Handle<D> {
        &mut self.handle
    }

    pub fn poll_fd(&self) -> RawFd {
        self.poll_fd
    }

    pub fn dispatch_client(
        &mut self,
        data: &mut D,
        client_id: InnerClientId,
    ) -> std::io::Result<usize> {
        let ret = self.dispatch_events_for(data, client_id);
        self.handle.handle.cleanup(data);
        ret
    }

    #[cfg(target_os = "linux")]
    pub fn dispatch_all_clients(&mut self, data: &mut D) -> std::io::Result<usize> {
        let mut dispatched = 0;
        loop {
            let mut events = [EpollEvent::empty(); 32];
            let nevents = epoll_wait(self.poll_fd, &mut events, 0)?;

            if nevents == 0 {
                break;
            }

            for event in events.iter().take(nevents) {
                let id = InnerClientId::from_u64(event.data());
                // remove the cb while we call it, to gracefully handle reentrancy
                if let Ok(count) = self.dispatch_events_for(data, id) {
                    dispatched += count;
                }
            }
            self.handle.handle.cleanup(data);
        }

        Ok(dispatched)
    }

    #[cfg(any(
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    pub fn dispatch_all_clients(&mut self, data: &mut D) -> std::io::Result<usize> {
        let mut dispatched = 0;
        loop {
            let mut events = [KEvent::new(
                0,
                EventFilter::EVFILT_READ,
                EventFlag::empty(),
                FilterFlag::empty(),
                0,
                0,
            ); 32];

            let nevents = kevent(self.poll_fd, &[], &mut events, 0)?;

            if nevents == 0 {
                break;
            }

            for event in events.iter().take(nevents) {
                let id = InnerClientId::from_u64(event.udata() as u64);
                // remove the cb while we call it, to gracefully handle reentrancy
                if let Ok(count) = self.dispatch_events_for(data, id) {
                    dispatched += count;
                }
            }
            self.handle.handle.cleanup(data);
        }

        Ok(dispatched)
    }

    pub(crate) fn dispatch_events_for(
        &mut self,
        data: &mut D,
        client_id: InnerClientId,
    ) -> std::io::Result<usize> {
        let mut dispatched = 0;
        loop {
            let action =
                if let Ok(client) = self.handle.handle.clients.get_client_mut(client_id.clone()) {
                    let (message, object) = match client.next_request() {
                        Ok(v) => v,
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            if dispatched > 0 {
                                break;
                            } else {
                                return Err(e);
                            }
                        }
                        Err(e) => return Err(e),
                    };
                    dispatched += 1;
                    if same_interface(object.interface, &WL_DISPLAY_INTERFACE) {
                        client.handle_display_request(message, &mut self.handle.handle.registry);
                        continue;
                    } else if same_interface(object.interface, &WL_REGISTRY_INTERFACE) {
                        if let Some((client, global, object, handler)) = client
                            .handle_registry_request(message, &mut self.handle.handle.registry)
                        {
                            DispatchAction::Bind { client, global, object, handler }
                        } else {
                            continue;
                        }
                    } else {
                        let object_id = InnerObjectId {
                            id: message.sender_id,
                            serial: object.data.serial,
                            interface: object.interface,
                            client_id: client.id.clone(),
                        };
                        let opcode = message.opcode;
                        let (arguments, is_destructor, created_id) =
                            match client.process_request(&object, message) {
                                Some(args) => args,
                                None => continue,
                            };
                        // Return the whole set to invoke the callback while handle is not borrower via client
                        DispatchAction::Request {
                            object,
                            object_id,
                            opcode,
                            arguments,
                            is_destructor,
                            created_id,
                        }
                    }
                } else {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "Invalid client ID",
                    ));
                };
            match action {
                DispatchAction::Request {
                    object,
                    object_id,
                    opcode,
                    arguments,
                    is_destructor,
                    created_id,
                } => {
                    let ret = object.data.user_data.clone().request(
                        &mut self.handle,
                        data,
                        ClientId { id: client_id.clone() },
                        Message {
                            sender_id: ObjectId { id: object_id.clone() },
                            opcode,
                            args: arguments,
                        },
                    );
                    if is_destructor {
                        object.data.user_data.destroyed(
                            data,
                            ClientId { id: client_id.clone() },
                            ObjectId { id: object_id.clone() },
                        );
                        if let Ok(client) =
                            self.handle.handle.clients.get_client_mut(client_id.clone())
                        {
                            client.send_delete_id(object_id);
                        }
                    }
                    match (created_id, ret) {
                        (Some(child_id), Some(child_data)) => {
                            if let Ok(client) =
                                self.handle.handle.clients.get_client_mut(client_id.clone())
                            {
                                client
                                    .map
                                    .with(child_id.id, |obj| obj.data.user_data = child_data)
                                    .unwrap();
                            }
                        }
                        (None, None) => {}
                        (Some(child_id), None) => {
                            // Allow the callback to not return any data if the client is already dead (typically
                            // if the callback provoked a protocol error)
                            if let Ok(client) =
                                self.handle.handle.clients.get_client(client_id.clone())
                            {
                                if !client.killed {
                                    panic!(
                                        "Callback creating object {} did not provide any object data.",
                                        child_id
                                    );
                                }
                            }
                        }
                        (None, Some(_)) => {
                            panic!("An object data was returned from a callback not creating any object");
                        }
                    }
                }
                DispatchAction::Bind { object, client, global, handler } => {
                    let child_data = handler.bind(
                        &mut self.handle,
                        data,
                        ClientId { id: client.clone() },
                        GlobalId { id: global },
                        ObjectId { id: object.clone() },
                    );
                    if let Ok(client) = self.handle.handle.clients.get_client_mut(client.clone()) {
                        client.map.with(object.id, |obj| obj.data.user_data = child_data).unwrap();
                    }
                }
            }
        }
        Ok(dispatched)
    }
}

enum DispatchAction<D> {
    Request {
        object: Object<Data<D>>,
        object_id: InnerObjectId,
        opcode: u16,
        arguments: SmallVec<[Argument<ObjectId>; 4]>,
        is_destructor: bool,
        created_id: Option<InnerObjectId>,
    },
    Bind {
        object: InnerObjectId,
        client: InnerClientId,
        global: InnerGlobalId,
        handler: Arc<dyn GlobalHandler<D>>,
    },
}
