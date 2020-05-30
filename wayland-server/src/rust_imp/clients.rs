use std::cell::RefCell;
use std::ffi::CString;
use std::os::unix::io::{FromRawFd, IntoRawFd, RawFd};
use std::rc::Rc;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::thread::{self, ThreadId};

use nix::Result as NixResult;

use wayland_commons::debug;
use wayland_commons::map::{Object, ObjectMap, ObjectMetadata, SERVER_ID_LIMIT};
use wayland_commons::socket::{BufferedSocket, Socket};
use wayland_commons::wire::{Argument, ArgumentType, Message, MessageDesc, MessageParseError};
use wayland_commons::{smallvec, ThreadGuard};

use crate::{DispatchData, Interface, UserDataMap};

use super::event_loop_glue::{FdManager, Token};
use super::globals::GlobalManager;
use super::resources::{ObjectMeta, ResourceDestructor, ResourceInner};
use super::{Dispatched, WAYLAND_DEBUG};

#[derive(Clone, Debug)]
pub(crate) enum Error {
    Protocol,
    Parse(MessageParseError),
    Nix(::nix::Error),
}

type BoxedClientDestructor = Box<dyn FnMut(Arc<UserDataMap>, DispatchData<'_>)>;

pub(crate) struct ClientConnection {
    socket: BufferedSocket,
    pub(crate) map: Arc<Mutex<ObjectMap<ObjectMeta>>>,
    user_data_map: Arc<UserDataMap>,
    destructors: ThreadGuard<Vec<BoxedClientDestructor>>,
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
            user_data_map: Arc::new(UserDataMap::new()),
            destructors: ThreadGuard::new(Vec::new()),
            last_error: None,
            pending_destructors: Vec::new(),
            zombie_clients: zombies,
        }
    }

    pub(crate) fn schedule_destructor(&mut self, resource: ResourceInner) {
        self.pending_destructors.push(resource);
    }

    pub(crate) fn call_destructors(&mut self, mut data: crate::DispatchData) {
        for resource in self.pending_destructors.drain(..) {
            if let Some(ref dest) = resource.object.meta.destructor {
                (&mut *dest.get().borrow_mut())(resource.clone(), data.reborrow());
            }
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

        if id < SERVER_ID_LIMIT {
            self.write_message(&Message {
                sender_id: 1,
                opcode: 1,
                args: smallvec![Argument::Uint(id)],
            })
        } else {
            Ok(())
        }
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
            map.find(id).and_then(|o| o.requests.get(opcode as usize)).map(|desc| desc.signature)
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
            let new_id = msg
                .args
                .iter()
                .flat_map(|a| if let Argument::NewId(nid) = *a { Some(nid) } else { None })
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
                    || !msg.args.iter().any(|a| a.get_type() == ArgumentType::NewId)
            );
        }

        Ok(Some(msg))
    }

    fn cleanup(mut self, mut data: crate::DispatchData) {
        let dummy_client = ClientInner {
            data: Arc::new(Mutex::new(None)),
            user_data_map: self.user_data_map.clone(),
            loop_thread: thread::current().id(),
        };
        self.map.lock().unwrap().with_all(|id, obj| {
            let resource = ResourceInner { id, object: obj.clone(), client: dummy_client.clone() };
            obj.meta.alive.store(false, Ordering::Release);
            if let Some(ref dest) = obj.meta.destructor {
                (&mut *dest.get().borrow_mut())(resource, data.reborrow());
            }
        });
        let _ = ::nix::unistd::close(self.socket.into_socket().into_raw_fd());
        for mut destructor in self.destructors.get_mut().drain(..) {
            destructor(self.user_data_map.clone(), data.reborrow());
        }
    }
}

#[derive(Clone)]
pub(crate) struct ClientInner {
    pub(crate) data: Arc<Mutex<Option<ClientConnection>>>,
    user_data_map: Arc<UserDataMap>,
    pub(crate) loop_thread: ThreadId,
}

impl ClientInner {
    pub(crate) fn alive(&self) -> bool {
        self.data.lock().unwrap().is_some()
    }

    pub(crate) fn equals(&self, other: &ClientInner) -> bool {
        Arc::ptr_eq(&self.data, &other.data)
    }

    pub(crate) fn flush(&self) {
        if let Some(ref mut cx) = *self.data.lock().unwrap() {
            let _ = cx.socket.flush();
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

    pub(crate) fn user_data_map(&self) -> &UserDataMap {
        &self.user_data_map
    }

    pub(crate) fn add_destructor<F: FnOnce(Arc<UserDataMap>, DispatchData<'_>) + 'static>(
        &self,
        destructor: F,
    ) {
        if self.loop_thread != std::thread::current().id() {
            panic!("Can only add a destructor from the thread hosting the Display.");
        }
        if let Some(ref mut client_data) = *self.data.lock().unwrap() {
            // Wrap the FnOnce in an FnMut because Box<FnOnce()> does not work
            // currently =(
            let mut opt_dest = Some(destructor);
            client_data.destructors.get_mut().push(Box::new(move |data_map, data| {
                if let Some(dest) = opt_dest.take() {
                    dest(data_map, data);
                }
            }));
        }
    }

    pub(crate) fn post_error(&self, object: u32, error_code: u32, msg: String) {
        if let Some(ref mut data) = *self.data.lock().unwrap() {
            let _ = data.write_message(&Message {
                sender_id: 1,
                opcode: 0,
                args: smallvec![
                    Argument::Object(object),
                    Argument::Uint(error_code),
                    Argument::Str(Box::new(CString::new(msg).unwrap())),
                ],
            });
        }
        self.kill();
    }

    pub(crate) fn create_resource<I: Interface>(&self, version: u32) -> Option<ResourceInner> {
        if self.loop_thread != thread::current().id() {
            panic!("Can only create ressources from the thread hosting the Display.");
        }
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
        Some(ResourceInner::from_id(id, map, self.clone()).unwrap())
    }

    pub(crate) fn set_dispatcher_for(
        &self,
        id: u32,
        dispatcher: Arc<ThreadGuard<RefCell<dyn super::Dispatcher>>>,
    ) {
        let guard = self.data.lock().unwrap();
        if let Some(ref cx) = *guard {
            let _ = cx.map.lock().unwrap().with(id, move |obj| {
                obj.meta.dispatcher = dispatcher;
            });
        }
    }

    pub(crate) fn set_destructor_for(
        &self,
        id: u32,
        destructor: Arc<ThreadGuard<ResourceDestructor>>,
    ) {
        let guard = self.data.lock().unwrap();
        if let Some(ref cx) = *guard {
            let _ = cx.map.lock().unwrap().with(id, move |obj| {
                obj.meta.destructor = Some(destructor);
            });
        }
    }
}

pub(crate) struct ClientManager {
    epoll_mgr: Rc<FdManager>,
    clients: Vec<(RefCell<Option<Token>>, ClientInner)>,
    zombie_clients: Arc<Mutex<Vec<ClientConnection>>>,
    global_mgr: Rc<RefCell<GlobalManager>>,
}

impl ClientManager {
    pub(crate) fn new(
        epoll_mgr: Rc<FdManager>,
        global_mgr: Rc<RefCell<GlobalManager>>,
    ) -> ClientManager {
        ClientManager {
            epoll_mgr,
            clients: Vec::new(),
            zombie_clients: Arc::new(Mutex::new(Vec::new())),
            global_mgr,
        }
    }

    pub(crate) unsafe fn init_client(
        &mut self,
        fd: RawFd,
        data: crate::DispatchData,
    ) -> ClientInner {
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
        let user_data_map = cx.user_data_map.clone();

        let client = ClientInner {
            data: Arc::new(Mutex::new(Some(cx))),
            user_data_map,
            loop_thread: thread::current().id(), // init_client is only called by the display, which does not change threads
        };

        let implementation = ClientImplementation { inner: client.clone(), map };

        // process any pending messages before inserting it into the event loop
        implementation.process_messages(data);

        if !client.alive() {
            // client already made a protocol error and we killed it, there is no point
            // inserting it in the event loop
            return client;
        }

        let source =
            match self.epoll_mgr.register(fd, move |data| implementation.process_messages(data)) {
                Ok(source) => Some(source),
                Err(e) => {
                    eprintln!("[wayland-server] Failed to insert client into event loop: {:?}", e);
                    client.kill();
                    None
                }
            };

        if source.is_some() {
            self.clients.push((RefCell::new(source), client.clone()));
        }

        client
    }

    pub(crate) fn flush_all(&mut self, mut disp_data: crate::DispatchData) {
        // flush all clients and cleanup dead ones
        let epoll_mgr = self.epoll_mgr.clone();
        self.clients.retain(|&(ref s, ref c)| {
            if let Some(ref mut data) = *c.data.lock().unwrap() {
                data.call_destructors(disp_data.reborrow());
                data.flush().is_ok()
            } else {
                // This is a dead client, clean it up
                if let Some(token) = s.borrow_mut().take() {
                    epoll_mgr.deregister(token);
                }
                false
            }
        });

        let mut guard = self.zombie_clients.lock().unwrap();
        for zombie in guard.drain(..) {
            zombie.cleanup(disp_data.reborrow());
        }
    }

    // kill & cleanup all clients
    pub(crate) fn kill_all(&mut self) {
        for &(_, ref client) in &self.clients {
            client.kill();
        }
        self.flush_all(crate::DispatchData::wrap(&mut ()));
    }
}

const DISPLAY_REQUESTS: &[MessageDesc] = &[
    MessageDesc { name: "sync", since: 1, signature: &[ArgumentType::NewId], destructor: false },
    MessageDesc {
        name: "get_registry",
        since: 1,
        signature: &[ArgumentType::NewId],
        destructor: false,
    },
];

const DISPLAY_EVENTS: &[MessageDesc] = &[
    MessageDesc {
        name: "error",
        since: 1,
        signature: &[ArgumentType::Object, ArgumentType::Uint, ArgumentType::Str],
        destructor: false,
    },
    MessageDesc {
        name: "delete_id",
        since: 1,
        signature: &[ArgumentType::Uint],
        destructor: false,
    },
];

const REGISTRY_REQUESTS: &[MessageDesc] = &[MessageDesc {
    name: "bind",
    since: 1,
    signature: &[ArgumentType::Uint, ArgumentType::Str, ArgumentType::Uint, ArgumentType::NewId],
    destructor: false,
}];

const REGISTRY_EVENTS: &[MessageDesc] = &[
    MessageDesc {
        name: "global",
        since: 1,
        signature: &[ArgumentType::Uint, ArgumentType::Str, ArgumentType::Uint],
        destructor: false,
    },
    MessageDesc {
        name: "global_remove",
        since: 1,
        signature: &[ArgumentType::Uint],
        destructor: false,
    },
];

fn display_req_child(opcode: u16, _: u32, meta: &ObjectMeta) -> Option<Object<ObjectMeta>> {
    match opcode {
        // sync
        0 => Some(Object::from_interface::<crate::protocol::wl_callback::WlCallback>(
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
    fn process_messages(&self, mut data: crate::DispatchData) {
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
                    let mut resourcemap =
                        super::ResourceMap::make(self.map.clone(), self.inner.clone());
                    let id = msg.sender_id;
                    let opcode = msg.opcode;
                    if let Some(res) =
                        ResourceInner::from_id(id, self.map.clone(), self.inner.clone())
                    {
                        let object = res.object.clone();
                        let mut dispatcher = object.meta.dispatcher.get().borrow_mut();
                        match dispatcher.dispatch(msg, res, &mut resourcemap, data.reborrow()) {
                            Dispatched::Yes => {}
                            Dispatched::NoDispatch(_msg, _res) => {
                                eprintln!(
                                    "[wayland-server] Request received for an object \
                                    not associated to any filter: {}@{}",
                                    object.interface, id
                                );
                                self.inner.post_error(
                                    1,
                                    super::display::DISPLAY_ERROR_NO_MEMORY,
                                    "Server-side bug, sorry.".into(),
                                );
                            }
                            Dispatched::BadMsg => {
                                self.inner.post_error(
                                    1,
                                    super::display::DISPLAY_ERROR_INVALID_METHOD,
                                    format!(
                                        "invalid method {}, object {}@{}",
                                        opcode, object.interface, id
                                    ),
                                );
                                return;
                            }
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

struct DisplayDispatcher {
    global_mgr: Rc<RefCell<GlobalManager>>,
}

impl super::Dispatcher for DisplayDispatcher {
    fn dispatch(
        &mut self,
        msg: Message,
        _resource: ResourceInner,
        map: &mut super::ResourceMap,
        _data: crate::DispatchData,
    ) -> Dispatched {
        use crate::protocol::wl_callback;

        if WAYLAND_DEBUG.load(Ordering::Relaxed) {
            debug::print_dispatched_message(
                "wl_display",
                1,
                DISPLAY_REQUESTS[msg.opcode as usize].name,
                &msg.args,
            );
        }

        match msg.opcode {
            // sync
            0 => {
                if let Some(&Argument::NewId(new_id)) = msg.args.first() {
                    if let Some(cb) = map.get_new::<wl_callback::WlCallback>(new_id) {
                        // TODO: send a more meaningful serial ?
                        cb.as_ref().send(wl_callback::Event::Done { callback_data: 0 });
                    } else {
                        return Dispatched::BadMsg;
                    }
                } else {
                    return Dispatched::BadMsg;
                }
            }
            // get_registry
            1 => {
                if let Some(&Argument::NewId(new_id)) = msg.args.first() {
                    // we don't have a regular object for the registry, rather we insert the
                    // dispatcher by hand
                    if let Err(()) = map.map.lock().unwrap().with(new_id, |obj| {
                        obj.meta.dispatcher =
                            Arc::new(ThreadGuard::new(RefCell::new(RegistryDispatcher {
                                global_mgr: self.global_mgr.clone(),
                            })));
                    }) {
                        return Dispatched::BadMsg;
                    }
                    self.global_mgr.borrow_mut().new_registry(new_id, map.client.clone());
                } else {
                    return Dispatched::BadMsg;
                }
            }
            _ => return Dispatched::BadMsg,
        }
        Dispatched::Yes
    }
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
        data: crate::DispatchData,
    ) -> Dispatched {
        if WAYLAND_DEBUG.load(Ordering::Relaxed) {
            debug::print_dispatched_message(
                "wl_registry",
                resource.id,
                REGISTRY_REQUESTS[msg.opcode as usize].name,
                &msg.args,
            );
        }

        let mut iter = msg.args.into_iter();
        let global_id = match iter.next() {
            Some(Argument::Uint(u)) => u,
            _ => return Dispatched::BadMsg,
        };
        let interface = match iter.next() {
            Some(Argument::Str(s)) => s,
            _ => return Dispatched::BadMsg,
        };
        let version = match iter.next() {
            Some(Argument::Uint(u)) => u,
            _ => return Dispatched::BadMsg,
        };
        let new_id = match iter.next() {
            Some(Argument::NewId(id)) => id,
            _ => return Dispatched::BadMsg,
        };
        match self.global_mgr.borrow().bind(
            resource.id,
            new_id,
            global_id,
            &interface.to_string_lossy(),
            version,
            map.client.clone(),
            data,
        ) {
            Ok(()) => Dispatched::Yes,
            Err(()) => Dispatched::BadMsg,
        }
    }
}
