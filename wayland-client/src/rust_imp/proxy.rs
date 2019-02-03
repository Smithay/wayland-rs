use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, ThreadId};

use wayland_commons::map::{Object, ObjectMap, ObjectMetadata};
use wayland_commons::utils::UserData;
use wayland_commons::wire::{Argument, ArgumentType};
use wayland_commons::MessageGroup;

use super::connection::Connection;
use super::queues::QueueBuffer;
use super::{Dispatcher, EventQueueInner};
use {Interface, Proxy};

#[derive(Clone)]
pub(crate) struct ObjectMeta {
    pub(crate) buffer: QueueBuffer,
    pub(crate) alive: Arc<AtomicBool>,
    user_data: Arc<UserData>,
    pub(crate) dispatcher: Arc<Mutex<Dispatcher>>,
    pub(crate) server_destroyed: bool,
    pub(crate) client_destroyed: bool,
    queue_thread: ThreadId,
}

impl ObjectMetadata for ObjectMeta {
    fn child(&self) -> ObjectMeta {
        ObjectMeta {
            buffer: self.buffer.clone(),
            alive: Arc::new(AtomicBool::new(true)),
            user_data: Arc::new(UserData::empty()),
            dispatcher: super::default_dispatcher(),
            server_destroyed: false,
            client_destroyed: false,
            queue_thread: self.queue_thread,
        }
    }
}

impl ObjectMeta {
    pub(crate) fn new(buffer: QueueBuffer) -> ObjectMeta {
        ObjectMeta {
            buffer,
            alive: Arc::new(AtomicBool::new(true)),
            user_data: Arc::new(UserData::empty()),
            dispatcher: super::default_dispatcher(),
            server_destroyed: false,
            client_destroyed: false,
            queue_thread: thread::current().id(),
        }
    }

    fn dead() -> ObjectMeta {
        ObjectMeta {
            buffer: super::queues::create_queue_buffer(),
            alive: Arc::new(AtomicBool::new(false)),
            user_data: Arc::new(UserData::empty()),
            dispatcher: super::default_dispatcher(),
            server_destroyed: true,
            client_destroyed: true,
            queue_thread: thread::current().id(),
        }
    }
}

#[derive(Clone)]
pub(crate) struct ProxyInner {
    pub(crate) map: Arc<Mutex<ObjectMap<ObjectMeta>>>,
    pub(crate) connection: Arc<Mutex<Connection>>,
    pub(crate) object: Object<ObjectMeta>,
    pub(crate) id: u32,
}

impl ProxyInner {
    pub(crate) fn from_id(
        id: u32,
        map: Arc<Mutex<ObjectMap<ObjectMeta>>>,
        connection: Arc<Mutex<Connection>>,
    ) -> Option<ProxyInner> {
        let me = map.lock().unwrap().find(id);
        me.map(|obj| ProxyInner {
            map,
            connection,
            id,
            object: obj,
        })
    }

    pub(crate) fn is_interface<I: Interface>(&self) -> bool {
        self.object.is_interface::<I>()
    }

    pub(crate) fn is_alive(&self) -> bool {
        self.object.meta.alive.load(Ordering::Acquire)
    }

    pub fn version(&self) -> u32 {
        self.object.version
    }

    pub(crate) fn id(&self) -> u32 {
        if self.is_alive() {
            self.id
        } else {
            0
        }
    }

    pub(crate) fn get_user_data<UD: 'static>(&self) -> Option<&UD> {
        self.object.meta.user_data.get::<UD>()
    }

    pub(crate) fn send<I: Interface>(&self, msg: I::Request) {
        // grab the connection lock before anything else
        // this avoids the risk of marking ourselves dead while an other
        // thread is sending a message an accidentally sending that message
        // after ours if ours is a destructor
        let mut conn_lock = self.connection.lock().unwrap();
        let destructor = msg.is_destructor();
        let msg = msg.into_raw(self.id);
        if ::std::env::var_os("WAYLAND_DEBUG").is_some() {
            eprintln!(
                " -> {}@{}: {} {:?}",
                I::NAME,
                self.id,
                self.object.requests[msg.opcode as usize].name,
                msg.args
            );
        }
        // TODO: figure our if this can fail and still be recoverable ?
        conn_lock.write_message(&msg).expect("Sending a message failed.");
        if destructor {
            self.object.meta.alive.store(false, Ordering::Release);
            {
                // cleanup the map as appropriate
                let mut map = conn_lock.map.lock().unwrap();
                let server_destroyed = map
                    .with(self.id, |obj| {
                        obj.meta.client_destroyed = true;
                        obj.meta.server_destroyed
                    })
                    .unwrap_or(false);
                if server_destroyed {
                    map.remove(self.id);
                }
            }
        }
    }

    pub(crate) fn send_constructor<I, J>(
        &self,
        msg: I::Request,
        version: Option<u32>,
    ) -> Result<NewProxyInner, ()>
    where
        I: Interface,
        J: Interface,
    {
        // grab the connection lock before anything else
        // this avoids the risk or races during object creation
        let mut conn_lock = self.connection.lock().unwrap();
        let destructor = msg.is_destructor();
        let mut msg = msg.into_raw(self.id);
        if ::std::env::var_os("WAYLAND_DEBUG").is_some() {
            eprintln!(
                " -> {}@{}: {} {:?}",
                I::NAME,
                self.id,
                self.object.requests[msg.opcode as usize].name,
                msg.args
            );
        }

        let opcode = msg.opcode;

        // sanity check
        let mut nid_idx = I::Request::MESSAGES[opcode as usize]
            .signature
            .iter()
            .position(|&t| t == ArgumentType::NewId)
            .expect("Trying to use 'send_constructor' with a message not creating any object.");

        if let Some(o) = I::Request::child(opcode, 1, &()) {
            if !o.is_interface::<J>() {
                panic!("Trying to use 'send_constructor' with the wrong return type. Required interface {} but the message creates interface {}", J::NAME, o.interface)
            }
        } else {
            // there is no target interface in the protocol, this is a generic object-creating
            // function (likely wl_registry.bind), the newid arg will thus expand to (str, u32, obj)
            nid_idx += 2;
        }
        // insert the newly created object in the message
        let newproxy = match msg.args[nid_idx] {
            Argument::NewId(ref mut newid) => {
                let newp = match version {
                    Some(v) => self.child_versioned::<J>(v),
                    None => self.child::<J>(),
                };
                *newid = newp.id;
                newp
            }
            _ => unreachable!(),
        };

        conn_lock.write_message(&msg).expect("Sending a message failed.");
        if destructor {
            self.object.meta.alive.store(false, Ordering::Release);
            {
                // cleanup the map as appropriate
                let mut map = conn_lock.map.lock().unwrap();
                let server_destroyed = map
                    .with(self.id, |obj| {
                        obj.meta.client_destroyed = true;
                        obj.meta.server_destroyed
                    })
                    .unwrap_or(false);
                if server_destroyed {
                    map.remove(self.id);
                }
            }
        }

        Ok(newproxy)
    }

    pub(crate) fn equals(&self, other: &ProxyInner) -> bool {
        self.is_alive() && Arc::ptr_eq(&self.object.meta.alive, &other.object.meta.alive)
    }

    pub(crate) fn make_wrapper(&self, queue: &EventQueueInner) -> Result<ProxyInner, ()> {
        let mut wrapper = self.clone();
        wrapper.object.meta.buffer = queue.buffer.clone();
        // EventQueueInner is not Send so we must be in the right thread
        wrapper.object.meta.queue_thread = thread::current().id();
        Ok(wrapper)
    }

    pub(crate) fn child<I: Interface>(&self) -> NewProxyInner {
        self.child_versioned::<I>(self.object.version)
    }

    pub(crate) fn child_versioned<I: Interface>(&self, version: u32) -> NewProxyInner {
        let new_object = Object::from_interface::<I>(version, self.object.meta.child());
        let new_id = self.map.lock().unwrap().client_insert_new(new_object);
        NewProxyInner {
            map: self.map.clone(),
            connection: self.connection.clone(),
            id: new_id,
        }
    }

    pub(crate) fn child_placeholder(&self) -> ProxyInner {
        ProxyInner {
            map: self.map.clone(),
            connection: self.connection.clone(),
            object: Object::placeholder(self.object.meta.child()),
            id: 0,
        }
    }
}

pub(crate) struct NewProxyInner {
    map: Arc<Mutex<ObjectMap<ObjectMeta>>>,
    connection: Arc<Mutex<Connection>>,
    id: u32,
}

impl NewProxyInner {
    pub(crate) fn from_id(
        id: u32,
        map: Arc<Mutex<ObjectMap<ObjectMeta>>>,
        connection: Arc<Mutex<Connection>>,
    ) -> Option<NewProxyInner> {
        if map.lock().unwrap().find(id).is_some() {
            Some(NewProxyInner { map, connection, id })
        } else {
            None
        }
    }

    pub(crate) fn is_queue_on_current_thread(&self) -> bool {
        self.map
            .lock()
            .unwrap()
            .find(self.id)
            .map(|obj| obj.meta.queue_thread == thread::current().id())
            .unwrap_or(false)
    }

    // Invariants: Impl is either `Send` or we are on the same thread as the target event loop
    pub(crate) unsafe fn implement<I: Interface, F>(
        self,
        implementation: F,
        user_data: UserData,
    ) -> ProxyInner
    where
        F: FnMut(I::Event, I) + 'static,
        I: From<Proxy<I>>,
        I::Event: MessageGroup<Map = super::ProxyMap>,
    {
        let object = self.map.lock().unwrap().with(self.id, |obj| {
            obj.meta.dispatcher = super::make_dispatcher(implementation);
            obj.meta.user_data = Arc::new(user_data);
            obj.clone()
        });

        let object = match object {
            Ok(obj) => obj,
            Err(()) => {
                // We are trying to implement a non-existent object
                // This is either a bug in the lib (a NewProxy was created while it should not
                // have been possible) or an object was created and the server destroyed it
                // before it could be implemented.
                // Thus, we just create a dummy already-dead Proxy
                Object::from_interface::<I>(1, ObjectMeta::dead())
            }
        };

        ProxyInner {
            map: self.map,
            connection: self.connection,
            id: self.id,
            object,
        }
    }
}
