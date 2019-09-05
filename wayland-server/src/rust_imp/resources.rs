use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::{Interface, Resource};

use wayland_commons::map::{Object, ObjectMap, ObjectMetadata};
use wayland_commons::user_data::UserData;
use wayland_commons::{MessageGroup, ThreadGuard};

use super::{ClientInner, Dispatcher};

#[derive(Clone)]
pub(crate) struct ObjectMeta {
    pub(crate) dispatcher: Arc<ThreadGuard<RefCell<dyn Dispatcher>>>,
    pub(crate) destructor: Option<Arc<ThreadGuard<RefCell<dyn FnMut(ResourceInner)>>>>,
    pub(crate) alive: Arc<AtomicBool>,
    user_data: Arc<UserData>,
}

impl ObjectMetadata for ObjectMeta {
    fn child(&self) -> ObjectMeta {
        ObjectMeta::new()
    }
}

impl ObjectMeta {
    pub(crate) fn new() -> ObjectMeta {
        ObjectMeta {
            alive: Arc::new(AtomicBool::new(true)),
            user_data: Arc::new(UserData::new()),
            dispatcher: super::default_dispatcher(),
            destructor: None,
        }
    }

    pub(crate) fn with_dispatcher<D: Dispatcher>(disp: D) -> ObjectMeta {
        ObjectMeta {
            alive: Arc::new(AtomicBool::new(true)),
            user_data: Arc::new(UserData::new()),
            dispatcher: Arc::new(ThreadGuard::new(RefCell::new(disp))),
            destructor: None,
        }
    }
}

#[derive(Clone)]
pub(crate) struct ResourceInner {
    pub(crate) id: u32,
    pub(crate) object: Object<ObjectMeta>,
    pub(crate) client: ClientInner,
}

impl ResourceInner {
    pub(crate) fn from_id(
        id: u32,
        map: Arc<Mutex<ObjectMap<ObjectMeta>>>,
        client: ClientInner,
    ) -> Option<ResourceInner> {
        let me = map.lock().unwrap().find(id);
        me.map(|obj| ResourceInner {
            id,
            object: obj,
            client,
        })
    }

    pub(crate) fn is_interface<I: Interface>(&self) -> bool {
        self.object.is_interface::<I>()
    }

    pub(crate) fn send<I: Interface>(&self, msg: I::Event) {
        if let Some(ref mut conn_lock) = *self.client.data.lock().unwrap() {
            if !self.is_alive() {
                return;
            }
            let destructor = msg.is_destructor();
            let msg = msg.into_raw(self.id);
            if ::std::env::var_os("WAYLAND_DEBUG").is_some() {
                eprintln!(
                    " -> {}@{}: {} {:?}",
                    I::NAME,
                    self.id,
                    self.object.events[msg.opcode as usize].name,
                    msg.args
                );
            }
            // TODO: figure our if this can fail and still be recoverable ?
            conn_lock.write_message(&msg).expect("Sending a message failed.");
            if destructor {
                self.object.meta.alive.store(false, Ordering::Release);
                // schedule a destructor
                conn_lock.schedule_destructor(self.clone());
                // send delete_id
                let _ = conn_lock.delete_id(self.id);
            }
        }
    }

    pub(crate) fn is_alive(&self) -> bool {
        self.object.meta.alive.load(Ordering::Acquire)
    }

    pub(crate) fn version(&self) -> u32 {
        self.object.version
    }

    pub(crate) fn equals(&self, other: &ResourceInner) -> bool {
        self.is_alive() && Arc::ptr_eq(&self.object.meta.alive, &other.object.meta.alive)
    }

    pub(crate) fn same_client_as(&self, other: &ResourceInner) -> bool {
        self.client.equals(&other.client)
    }

    pub(crate) fn post_error(&self, error_code: u32, msg: String) {
        self.client.post_error(self.id, error_code, msg)
    }

    pub(crate) fn user_data(&self) -> &Arc<UserData> {
        &self.object.meta.user_data
    }

    pub(crate) fn client(&self) -> Option<ClientInner> {
        Some(self.client.clone())
    }

    pub(crate) fn id(&self) -> u32 {
        if self.is_alive() {
            self.id
        } else {
            0
        }
    }

    pub fn assign<I, E>(&self, filter: crate::Filter<E>)
    where
        I: Interface + AsRef<Resource<I>> + From<Resource<I>>,
        E: From<(Resource<I>, I::Request)> + 'static,
        I::Request: MessageGroup<Map = super::ResourceMap>,
    {
        self.client
            .set_dispatcher_for(self.id, super::make_dispatcher(filter));
    }

    pub fn assign_destructor<I, E>(&self, filter: crate::Filter<E>)
    where
        I: Interface + AsRef<Resource<I>> + From<Resource<I>>,
        E: From<Resource<I>> + 'static,
    {
        self.client
            .set_destructor_for(self.id, super::make_destructor(filter));
    }
}
