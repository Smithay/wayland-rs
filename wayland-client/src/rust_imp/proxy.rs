use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};
use std::sync::{Arc, Mutex};

use wayland_commons::map::{Object, ObjectMap, ObjectMetadata};
use wayland_commons::MessageGroup;

use super::connection::{Connection, Error as CError};
use super::queues::QueueBuffer;
use super::{Dispatcher, EventQueueInner};
use {Implementation, Interface, Proxy};

#[derive(Clone)]
pub(crate) struct ObjectMeta {
    pub(crate) buffer: QueueBuffer,
    pub(crate) alive: Arc<AtomicBool>,
    pub(crate) user_data: Arc<AtomicPtr<()>>,
    pub(crate) dispatcher: Arc<Mutex<Dispatcher>>,
}

impl ObjectMetadata for ObjectMeta {
    fn child(&self) -> ObjectMeta {
        ObjectMeta {
            buffer: self.buffer.clone(),
            alive: Arc::new(AtomicBool::new(true)),
            user_data: Arc::new(AtomicPtr::new(::std::ptr::null_mut())),
            dispatcher: super::default_dispatcher(),
        }
    }
}

impl ObjectMeta {
    pub(crate) fn new(buffer: QueueBuffer) -> ObjectMeta {
        ObjectMeta {
            buffer,
            alive: Arc::new(AtomicBool::new(true)),
            user_data: Arc::new(AtomicPtr::new(::std::ptr::null_mut())),
            dispatcher: super::default_dispatcher(),
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

    pub fn set_user_data(&self, ptr: *mut ()) {
        self.object.meta.user_data.store(ptr, Ordering::Release)
    }

    pub fn get_user_data(&self) -> *mut () {
        self.object.meta.user_data.load(Ordering::Release)
    }

    pub(crate) fn send<I: Interface>(&self, msg: I::Request) {
        // grab the connection lock before anything else
        // this avoids the risk of marking ourselve dead while an other
        // thread is sending a message an accidentaly sending that message
        // after ours if ours is a destructor
        let mut conn_lock = self.connection.lock().unwrap();
        let mut map_lock = self.map.lock().unwrap();
        if !self.is_alive() {
            return;
        }
        let destructor = msg.is_destructor();
        // TODO: figure our if this can fail and still be recoverable ?
        let _ = conn_lock
            .write_message(&msg.into_raw(self.id))
            .expect("Sending a message failed.");
        if destructor {
            self.object.meta.alive.store(false, Ordering::Release);
            map_lock.kill(self.id);
        }
    }

    pub(crate) fn equals(&self, other: &ProxyInner) -> bool {
        self.is_alive() && other.is_alive() && self.id == other.id
    }

    pub(crate) fn make_wrapper(&self, queue: &EventQueueInner) -> Result<ProxyInner, ()> {
        let mut wrapper = self.clone();
        wrapper.object.meta.buffer = queue.buffer.clone();
        Ok(wrapper)
    }

    pub(crate) fn is_implemented_with<I: Interface, Impl>(&self) -> bool
    where
        Impl: Implementation<Proxy<I>, I::Event> + 'static,
        I::Event: MessageGroup<Map = super::ProxyMap>,
    {
        self.object
            .meta
            .dispatcher
            .lock()
            .unwrap()
            .is::<super::ImplDispatcher<I, Impl>>()
    }

    pub(crate) fn child<I: Interface>(&self) -> NewProxyInner {
        self.child_versioned::<I>(self.object.version)
    }

    pub fn child_versioned<I: Interface>(&self, version: u32) -> NewProxyInner {
        let new_object = Object::from_interface::<I>(version, self.object.meta.child());
        let new_id = self.map.lock().unwrap().client_insert_new(new_object);
        NewProxyInner {
            map: self.map.clone(),
            connection: self.connection.clone(),
            id: new_id,
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

    // Invariants: Impl is either `Send` or we are on the same thread as the target event loop
    pub(crate) unsafe fn implement<I: Interface, Impl>(self, implementation: Impl) -> ProxyInner
    where
        Impl: Implementation<Proxy<I>, I::Event> + 'static,
        I::Event: MessageGroup<Map = super::ProxyMap>,
    {
        let object = {
            let mut map_lock = self.map.lock().unwrap();
            map_lock.with_meta(self.id, |meta| {
                meta.dispatcher = super::make_dispatcher(implementation)
            });
            // The object cannot be dead (or recycled), because for it to occur we would have
            // received a message destroying it, which would have caused a panic by the default
            // dispatcher if the proxy was not yet implemented.
            // As a result, if this .expect() triggers, there is a bug in the lib.
            map_lock
                .find_alive(self.id)
                .expect("Trying to implement a dead object!")
        };
        ProxyInner {
            map: self.map,
            connection: self.connection,
            id: self.id,
            object,
        }
    }
}
