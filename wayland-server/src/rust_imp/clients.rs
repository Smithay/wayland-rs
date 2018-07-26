use std::cell::RefCell;
use std::ffi::CString;
use std::os::unix::io::{FromRawFd, IntoRawFd, RawFd};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use mio::Ready;

use nix::Result as NixResult;

use wayland_commons::map::{Object, ObjectMap, ObjectMetadata};
use wayland_commons::socket::{BufferedSocket, Socket};
use wayland_commons::wire::{Argument, ArgumentType, Message, MessageDesc, MessageParseError};

use sources::{FdEvent, FdInterest};
use {Implementation, Interface};

use super::event_loop::SourcesPoll;
use super::globals::GlobalManager;
use super::resources::{NewResourceInner, ObjectMeta, ResourceInner};
use super::SourceInner;

#[derive(Clone, Debug)]
pub(crate) enum Error {
    Protocol,
    Parse(MessageParseError),
    Nix(::nix::Error),
}

struct UserData(*mut ());

unsafe impl Send for UserData {}

pub(crate) struct ClientConnection {
    socket: BufferedSocket,
    pub(crate) map: Arc<Mutex<ObjectMap<ObjectMeta>>>,
    user_data: UserData,
    destructor: Option<fn(*mut ())>,
    last_error: Option<Error>,
    pending_destructors: Vec<ResourceInner>,
    zombie_clients: Arc<Mutex<Vec<ClientConnection>>>,
}

impl ClientConnection {
    unsafe fn new(
        fd: RawFd,
        display_object: Object<ObjectMeta>,
        zombies: Arc<Mutex<Vec<ClientConnection>>>,
    ) -> ClientConnection {
        let socket = BufferedSocket::new(Socket::from_raw_fd(fd));

        let mut map = ObjectMap::new();
        // Insert first pre-existing object
        map.insert_at(1, display_object).unwrap();

        ClientConnection {
            socket,
            map: Arc::new(Mutex::new(map)),
            user_data: UserData(::std::ptr::null_mut()),
            destructor: None,
            last_error: None,
            pending_destructors: Vec::new(),
            zombie_clients: zombies,
        }
    }

    pub(crate) fn schedule_destructor(&mut self, resource: ResourceInner) {
        self.pending_destructors.push(resource);
    }

    pub(crate) fn call_destructors(&mut self) {
        for resource in self.pending_destructors.drain(..) {
            resource
                .object
                .meta
                .dispatcher
                .lock()
                .unwrap()
                .destroy(resource.clone());
        }
    }

    pub(crate) fn write_message(&mut self, msg: &Message) -> NixResult<()> {
        self.socket.write_message(msg)
    }

    pub(crate) fn flush(&mut self) -> NixResult<()> {
        self.socket.flush()
    }

    pub(crate) fn delete_id(&mut self, id: u32) -> NixResult<()> {
        self.map.lock().unwrap().remove(id);

        self.write_message(&Message {
            sender_id: 1,
            opcode: 1,
            args: vec![Argument::Uint(id)],
        })
    }

    pub(crate) fn read_request(&mut self) -> Result<Option<Message>, Error> {
        if let Some(ref err) = self.last_error {
            return Err(err.clone());
        }
        // acquire the map lock, this means no objects can be created nor destroyed while we
        // are reading requests
        let mut map = self.map.lock().unwrap();
        // read messages
        let ret = self.socket.read_one_message(|id, opcode| {
            map.find(id)
                .and_then(|o| o.requests.get(opcode as usize))
                .map(|desc| desc.signature)
        });
        let msg = match ret {
            Ok(msg) => msg,
            Err(MessageParseError::Malformed) => {
                self.last_error = Some(Error::Parse(MessageParseError::Malformed));
                return Err(Error::Parse(MessageParseError::Malformed));
            }
            Err(MessageParseError::MissingData) | Err(MessageParseError::MissingFD) => {
                // missing data, read sockets and try again
                self.socket.fill_incoming_buffers().map_err(Error::Nix)?;
                match self.socket.read_one_message(|id, opcode| {
                    map.find(id)
                        .and_then(|o| o.requests.get(opcode as usize))
                        .map(|desc| desc.signature)
                }) {
                    Ok(msg) => msg,
                    Err(MessageParseError::Malformed) => {
                        self.last_error = Some(Error::Parse(MessageParseError::Malformed));
                        return Err(Error::Parse(MessageParseError::Malformed));
                    }
                    Err(MessageParseError::MissingData) | Err(MessageParseError::MissingFD) => {
                        // still nothing, there is nothing to read
                        return Ok(None);
                    }
                }
            }
        };

        // we reach here, there is now a message to process in msg

        // find the object that sent this message
        let object = match map.find(msg.sender_id) {
            Some(obj) => obj,
            None => {
                // this is a message sent to a destroyed object
                // to avoid dying because of races, we just consume it into void
                // closing any associated FDs
                for a in msg.args {
                    if let Argument::Fd(fd) = a {
                        let _ = ::nix::unistd::close(fd);
                    }
                }
                return Ok(None);
            }
        };

        // create a new object if applicable
        if let Some(child) = object.request_child(msg.opcode) {
            let new_id = msg.args
                .iter()
                .flat_map(|a| {
                    if let &Argument::NewId(nid) = a {
                        Some(nid)
                    } else {
                        None
                    }
                })
                .next()
                .unwrap();
            let child_interface = child.interface;
            if let Err(()) = map.insert_at(new_id, child) {
                eprintln!(
                    "[wayland-client] Protocol error: server tried to create an object \"{}\" with invalid id \"{}\".",
                    child_interface,
                    new_id
                );
                // abort parsing, this is an unrecoverable error
                self.last_error = Some(Error::Protocol);
                return Err(Error::Protocol);
            }
        } else {
            // debug assert: if this opcode does not define a child, then there should be no
            // NewId argument, unless we are the registry
            debug_assert!(
                object.interface == "wl_registry"
                    || msg.args.iter().any(|a| a.get_type() == ArgumentType::NewId) == false
            );
        }

        Ok(Some(msg))
    }

    fn cleanup(self) {
        let dummy_client = ClientInner {
            data: Arc::new(Mutex::new(None)),
        };
        self.map.lock().unwrap().with_all(|id, obj| {
            let resource = ResourceInner {
                id,
                object: obj.clone(),
                map: self.map.clone(),
                client: dummy_client.clone(),
            };
            obj.meta.dispatcher.lock().unwrap().destroy(resource);
        });
        let _ = ::nix::unistd::close(self.socket.into_socket().into_raw_fd());
        if let Some(destructor) = self.destructor {
            destructor(self.user_data.0);
        }
    }
}

#[derive(Clone)]
pub(crate) struct ClientInner {
    pub(crate) data: Arc<Mutex<Option<ClientConnection>>>,
}

impl ClientInner {
    pub(crate) fn alive(&self) -> bool {
        self.data.lock().unwrap().is_some()
    }

    pub(crate) fn equals(&self, other: &ClientInner) -> bool {
        Arc::ptr_eq(&self.data, &other.data)
    }

    pub(crate) fn flush(&self) {
        if let Some(ref mut data) = *self.data.lock().unwrap() {
            let _ = data.socket.flush();
        }
    }

    pub(crate) fn kill(&self) {
        if let Some(mut clientconn) = self.data.lock().unwrap().take() {
            let _ = clientconn.socket.flush();
            // call all objects destructors
            let zombies = clientconn.zombie_clients.clone();
            zombies.lock().unwrap().push(clientconn);
        }
    }

    pub(crate) fn set_user_data(&self, data: *mut ()) {
        if let Some(ref mut client_data) = *self.data.lock().unwrap() {
            client_data.user_data.0 = data;
        }
    }

    pub(crate) fn get_user_data(&self) -> *mut () {
        if let Some(ref mut client_data) = *self.data.lock().unwrap() {
            client_data.user_data.0
        } else {
            ::std::ptr::null_mut()
        }
    }

    pub(crate) fn set_destructor(&self, destructor: fn(*mut ())) {
        if let Some(ref mut client_data) = *self.data.lock().unwrap() {
            client_data.destructor = Some(destructor);
        }
    }

    pub(crate) fn post_error(&self, object: u32, error_code: u32, msg: String) {
        if let Some(ref mut data) = *self.data.lock().unwrap() {
            let _ = data.write_message(&Message {
                sender_id: 1,
                opcode: 0,
                args: vec![
                    Argument::Object(object),
                    Argument::Uint(error_code),
                    Argument::Str(CString::new(msg).unwrap()),
                ],
            });
        }
        self.kill();
    }

    pub(crate) fn create_resource<I: Interface>(&self, version: u32) -> Option<NewResourceInner> {
        let (id, map) = {
            if let Some(ref cx) = *self.data.lock().unwrap() {
                (
                    cx.map
                        .lock()
                        .unwrap()
                        .server_insert_new(Object::from_interface::<I>(version, ObjectMeta::new())),
                    cx.map.clone(),
                )
            } else {
                return None;
            }
        };
        Some(NewResourceInner::from_id(id, map, self.clone()).unwrap())
    }
}

pub(crate) struct ClientManager {
    sources_poll: SourcesPoll,
    clients: Vec<(RefCell<Option<SourceInner<FdEvent>>>, ClientInner)>,
    zombie_clients: Arc<Mutex<Vec<ClientConnection>>>,
    global_mgr: Rc<RefCell<GlobalManager>>,
}

impl ClientManager {
    pub(crate) fn new(sources_poll: SourcesPoll, global_mgr: Rc<RefCell<GlobalManager>>) -> ClientManager {
        ClientManager {
            sources_poll,
            clients: Vec::new(),
            zombie_clients: Arc::new(Mutex::new(Vec::new())),
            global_mgr,
        }
    }

    pub(crate) unsafe fn init_client(&mut self, fd: RawFd) -> ClientInner {
        let display_object = Object {
            interface: "wl_display",
            version: 1,
            requests: DISPLAY_REQUESTS,
            events: DISPLAY_EVENTS,
            meta: ObjectMeta::with_dispatcher(DisplayDispatcher {
                global_mgr: self.global_mgr.clone(),
            }),
            childs_from_events: no_child,
            childs_from_requests: display_req_child,
        };

        let cx = ClientConnection::new(fd, display_object, self.zombie_clients.clone());
        let map = cx.map.clone();

        let client = ClientInner {
            data: Arc::new(Mutex::new(Some(cx))),
        };

        let implementation = ClientImplementation {
            inner: client.clone(),
            map,
        };

        // process any pending messages before inserting it into the event loop
        implementation.process_messages();

        if !client.alive() {
            // client already made a protocol error and we killed it, there is no point
            // inserting it in the event loop
            return client;
        }

        let source = match self.sources_poll.insert_source(
            fd,
            Ready::readable(),
            implementation,
            FdEvent::Ready {
                fd,
                mask: FdInterest::READ,
            },
        ) {
            Ok(source) => Some(source),
            Err((e, _)) => {
                eprintln!(
                    "[wayland-server] Failed to insert client into event loop: {:?}",
                    e
                );
                client.kill();
                None
            }
        };

        if source.is_some() {
            self.clients.push((RefCell::new(source), client.clone()));
        }

        client
    }

    pub(crate) fn flush_all(&mut self) {
        // flush all clients and cleanup dead ones
        self.clients.retain(|&(ref s, ref c)| {
            if let Some(ref mut data) = *c.data.lock().unwrap() {
                data.call_destructors();
                data.flush().is_ok()
            } else {
                // This is a dead client, clean it up
                if let Some(source) = s.borrow_mut().take() {
                    source.remove();
                }
                false
            }
        });

        let mut guard = self.zombie_clients.lock().unwrap();
        for zombie in guard.drain(..) {
            zombie.cleanup();
        }
    }
}

const DISPLAY_REQUESTS: &'static [MessageDesc] = &[
    MessageDesc {
        name: "sync",
        since: 1,
        signature: &[ArgumentType::NewId],
    },
    MessageDesc {
        name: "get_registry",
        since: 1,
        signature: &[ArgumentType::NewId],
    },
];

const DISPLAY_EVENTS: &'static [MessageDesc] = &[
    MessageDesc {
        name: "error",
        since: 1,
        signature: &[ArgumentType::Object, ArgumentType::Uint, ArgumentType::Str],
    },
    MessageDesc {
        name: "delete_id",
        since: 1,
        signature: &[ArgumentType::Uint],
    },
];

const REGISTRY_REQUESTS: &'static [MessageDesc] = &[MessageDesc {
    name: "bind",
    since: 1,
    signature: &[
        ArgumentType::Uint,
        ArgumentType::Str,
        ArgumentType::Uint,
        ArgumentType::NewId,
    ],
}];

const REGISTRY_EVENTS: &'static [MessageDesc] = &[
    MessageDesc {
        name: "global",
        since: 1,
        signature: &[ArgumentType::Uint, ArgumentType::Str, ArgumentType::Uint],
    },
    MessageDesc {
        name: "global_remove",
        since: 1,
        signature: &[ArgumentType::Uint],
    },
];

fn display_req_child(opcode: u16, _: u32, meta: &ObjectMeta) -> Option<Object<ObjectMeta>> {
    match opcode {
        // sync
        0 => Some(Object::from_interface::<::protocol::wl_callback::WlCallback>(
            1,
            meta.child(),
        )),
        // registry
        1 => Some(Object {
            interface: "wl_registry",
            version: 1,
            requests: REGISTRY_REQUESTS,
            events: REGISTRY_EVENTS,
            meta: meta.child(),
            childs_from_events: no_child,
            childs_from_requests: no_child,
        }),
        _ => None,
    }
}

fn no_child(_: u16, _: u32, _: &ObjectMeta) -> Option<Object<ObjectMeta>> {
    None
}

struct ClientImplementation {
    inner: ClientInner,
    map: Arc<Mutex<ObjectMap<ObjectMeta>>>,
}

impl ClientImplementation {
    fn process_messages(&self) {
        loop {
            // we must process the messages one by one, because message parsing depends
            // on the contents of the object map, which each message can change...
            let ret = if let Some(ref mut data) = *self.inner.data.lock().unwrap() {
                data.read_request()
            } else {
                // client is now dead, abort
                return;
            };

            match ret {
                Ok(None) | Err(Error::Nix(::nix::Error::Sys(::nix::errno::Errno::EAGAIN))) => {
                    // nothing more to read
                    return;
                }
                Ok(Some(msg)) => {
                    // there is a message to dispatch
                    let mut resourcemap = super::ResourceMap::make(self.map.clone(), self.inner.clone());
                    let id = msg.sender_id;
                    let opcode = msg.opcode;
                    if let Some(res) = ResourceInner::from_id(id, self.map.clone(), self.inner.clone()) {
                        let object = res.object.clone();
                        let mut dispatcher = object.meta.dispatcher.lock().unwrap();
                        if let Err(()) = dispatcher.dispatch(msg, res, &mut resourcemap) {
                            self.inner.post_error(
                                1,
                                super::display::DISPLAY_ERROR_INVALID_METHOD,
                                format!("invalid method {}, object {}@{}", opcode, object.interface, id),
                            );
                            return;
                        }
                    } else {
                        self.inner.post_error(
                            1,
                            super::display::DISPLAY_ERROR_INVALID_OBJECT,
                            format!("invalid object {}", id),
                        );
                        return;
                    }
                }
                Err(_) => {
                    // on error, kill the client
                    self.inner.kill();
                    return;
                }
            }
        }
    }
}

impl Implementation<(), FdEvent> for ClientImplementation {
    fn receive(&mut self, event: FdEvent, (): ()) {
        match event {
            FdEvent::Ready { .. } => self.process_messages(),
            FdEvent::Error { .. } => {
                // in case of error, kill the client
                self.inner.kill();
            }
        }
    }
}

struct DisplayDispatcher {
    global_mgr: Rc<RefCell<GlobalManager>>,
}

impl super::Dispatcher for DisplayDispatcher {
    fn dispatch(
        &mut self,
        msg: Message,
        _resource: ResourceInner,
        map: &mut super::ResourceMap,
    ) -> Result<(), ()> {
        use protocol::wl_callback;
        match msg.opcode {
            // sync
            0 => if let Some(&Argument::NewId(new_id)) = msg.args.first() {
                if let Some(cb) = map.get_new::<wl_callback::WlCallback>(new_id) {
                    let cb = cb.implement(|r, _| match r {}, None::<fn(_, _)>);
                    // TODO: send a more meaningful serial ?
                    cb.send(wl_callback::Event::Done { callback_data: 0 });
                } else {
                    return Err(());
                }
            } else {
                return Err(());
            },
            // get_registry
            1 => if let Some(&Argument::NewId(new_id)) = msg.args.first() {
                // we don't have a regular object for the registry, rather we insert the
                // dispatcher by hand
                map.map.lock().unwrap().with(new_id, |obj| {
                    obj.meta.dispatcher = Arc::new(Mutex::new(RegistryDispatcher {
                        global_mgr: self.global_mgr.clone(),
                    }));
                })?;
                self.global_mgr
                    .borrow_mut()
                    .new_registry(new_id, map.client.clone());
            } else {
                return Err(());
            },
            _ => return Err(()),
        }
        Ok(())
    }

    fn destroy(&mut self, _resource: ResourceInner) {}
}

struct RegistryDispatcher {
    global_mgr: Rc<RefCell<GlobalManager>>,
}

impl super::Dispatcher for RegistryDispatcher {
    fn dispatch(
        &mut self,
        msg: Message,
        resource: ResourceInner,
        map: &mut super::ResourceMap,
    ) -> Result<(), ()> {
        let mut iter = msg.args.into_iter();
        let global_id = match iter.next() {
            Some(Argument::Uint(u)) => u,
            _ => return Err(()),
        };
        let interface = match iter.next() {
            Some(Argument::Str(s)) => s,
            _ => return Err(()),
        };
        let version = match iter.next() {
            Some(Argument::Uint(u)) => u,
            _ => return Err(()),
        };
        let new_id = match iter.next() {
            Some(Argument::NewId(id)) => id,
            _ => return Err(()),
        };
        self.global_mgr.borrow().bind(
            resource.id,
            new_id,
            global_id,
            &interface.to_string_lossy(),
            version,
            map.client.clone(),
        )
    }

    fn destroy(&mut self, _resource: ResourceInner) {}
}

// These unsafe impl is "technically wrong", but actually right for the same
// reasons as super::ImplDispatcher
unsafe impl Send for DisplayDispatcher {}
unsafe impl Send for RegistryDispatcher {}
