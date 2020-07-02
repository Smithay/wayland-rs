use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use wayland_commons::debug;
use wayland_commons::filter::Filter;
use wayland_commons::map::{Object, ObjectMap, ObjectMetadata};
use wayland_commons::user_data::UserData;
use wayland_commons::wire::{Argument, ArgumentType};
use wayland_commons::MessageGroup;

use super::connection::Connection;
use super::queues::QueueBuffer;
use super::{Dispatcher, EventQueueInner, WAYLAND_DEBUG};
use crate::{Interface, Main, Proxy};

#[derive(Clone)]
pub(crate) struct ObjectMeta {
    pub(crate) buffer: QueueBuffer,
    pub(crate) alive: Arc<AtomicBool>,
    user_data: Arc<UserData>,
    pub(crate) dispatcher: Arc<Mutex<dyn Dispatcher>>,
    pub(crate) server_destroyed: bool,
    pub(crate) client_destroyed: bool,
}

impl ObjectMetadata for ObjectMeta {
    fn child(&self) -> ObjectMeta {
        ObjectMeta {
            buffer: self.buffer.clone(),
            alive: Arc::new(AtomicBool::new(true)),
            user_data: Arc::new(UserData::new()),
            dispatcher: super::default_dispatcher(),
            server_destroyed: false,
            client_destroyed: false,
        }
    }
}

impl ObjectMeta {
    pub(crate) fn new(buffer: QueueBuffer) -> ObjectMeta {
        ObjectMeta {
            buffer,
            alive: Arc::new(AtomicBool::new(true)),
            user_data: Arc::new(UserData::new()),
            dispatcher: super::default_dispatcher(),
            server_destroyed: false,
            client_destroyed: false,
        }
    }

    fn dead() -> ObjectMeta {
        ObjectMeta {
            buffer: super::queues::create_queue_buffer(),
            alive: Arc::new(AtomicBool::new(false)),
            user_data: Arc::new(UserData::new()),
            dispatcher: super::default_dispatcher(),
            server_destroyed: true,
            client_destroyed: true,
        }
    }
}

#[derive(Clone)]
pub(crate) struct ProxyInner {
    pub(crate) map: Arc<Mutex<ObjectMap<ObjectMeta>>>,
    pub(crate) connection: Arc<Mutex<Connection>>,
    pub(crate) object: Object<ObjectMeta>,
    pub(crate) id: u32,
    pub(crate) queue: Option<QueueBuffer>,
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
            queue: Some(obj.meta.buffer.clone()),
            object: obj,
        })
    }

    pub(crate) fn dead<I: Interface>(
        id: u32,
        map: Arc<Mutex<ObjectMap<ObjectMeta>>>,
        connection: Arc<Mutex<Connection>>,
    ) -> ProxyInner {
        ProxyInner {
            map,
            connection,
            id,
            queue: None,
            object: Object::from_interface::<I>(1, ObjectMeta::dead()),
        }
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

    pub(crate) fn user_data(&self) -> &UserData {
        &*self.object.meta.user_data
    }

    pub(crate) fn detach(&mut self) {
        self.queue = None;
    }

    pub(crate) fn attach(&mut self, queue: &EventQueueInner) {
        self.queue = Some(queue.buffer.clone())
    }

    pub(crate) fn send<I, J>(&self, msg: I::Request, version: Option<u32>) -> Option<ProxyInner>
    where
        I: Interface,
        J: Interface,
    {
        // grab the connection lock before anything else
        // this avoids the risk or races during object creation
        let mut conn_lock = self.connection.lock().unwrap();
        let destructor = msg.is_destructor();
        let mut msg = msg.into_raw(self.id);

        let opcode = msg.opcode;

        // figure out if the call creates an object
        let nid_idx = I::Request::MESSAGES[opcode as usize]
            .signature
            .iter()
            .position(|&t| t == ArgumentType::NewId);

        let alive = self.is_alive();

        let ret = if let Some(mut nid_idx) = nid_idx {
            let target_queue = self
                .queue
                .clone()
                .expect("Attemping to create an object from a non-attached proxy.");
            if let Some(o) = I::Request::child(opcode, 1, &()) {
                if !o.is_interface::<J>() {
                    panic!(
                        "Trying to use 'send_constructor' with the wrong return type. \
                        Required interface {} but the message creates interface {}",
                        J::NAME,
                        o.interface
                    )
                }
            } else {
                // There is no target interface in the protocol, this is a generic object-creating
                // function (likely wl_registry.bind), the newid arg will thus expand to
                // (str, u32, obj).
                nid_idx += 2;
            }
            // insert the newly created object in the message
            let new_object = Object::from_interface::<J>(
                version.unwrap_or(self.object.version),
                if alive { ObjectMeta::new(target_queue.clone()) } else { ObjectMeta::dead() },
            );
            let mut new_id = 0;
            if alive {
                new_id = self.map.lock().unwrap().client_insert_new(new_object.clone());
                msg.args[nid_idx] = Argument::NewId(new_id);
            }
            Some(ProxyInner {
                map: self.map.clone(),
                connection: self.connection.clone(),
                id: new_id,
                object: new_object,
                queue: Some(target_queue),
            })
        } else {
            None
        };

        if WAYLAND_DEBUG.load(Ordering::Relaxed) {
            debug::print_send_message(
                I::NAME,
                self.id,
                alive,
                self.object.requests[msg.opcode as usize].name,
                &msg.args,
            );
        }

        // Only actually send the message (& process destructor) if the object is alive.
        if !alive {
            return ret;
        }

        conn_lock.write_message(&msg).expect("Sending a message failed.");

        if destructor {
            self.object.meta.alive.store(false, Ordering::Release);

            // Cleanup the map as appropriate.
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

        ret
    }

    pub(crate) fn equals(&self, other: &ProxyInner) -> bool {
        self.is_alive() && Arc::ptr_eq(&self.object.meta.alive, &other.object.meta.alive)
    }

    pub fn assign<I, E>(&self, filter: Filter<E>)
    where
        I: Interface + AsRef<Proxy<I>> + From<Proxy<I>> + Sync,
        E: From<(Main<I>, I::Event)> + 'static,
        I::Event: MessageGroup<Map = super::ProxyMap>,
    {
        // ignore failure if target object is dead
        let _ = self.map.lock().unwrap().with(self.id, |obj| {
            obj.meta.dispatcher = super::make_dispatcher(filter);
        });
    }
}
