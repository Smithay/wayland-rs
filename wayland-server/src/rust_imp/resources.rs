use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};
use std::sync::{Arc, Mutex};

use {Implementation, Interface, Resource};

use wayland_commons::map::{Object, ObjectMap, ObjectMetadata};
use wayland_commons::MessageGroup;

use super::{ClientInner, Dispatcher, EventLoopInner};

#[derive(Clone)]
pub(crate) struct ObjectMeta {
    pub(crate) dispatcher: Arc<Mutex<Dispatcher>>,
    pub(crate) alive: Arc<AtomicBool>,
    pub(crate) user_data: Arc<AtomicPtr<()>>,
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
            user_data: Arc::new(AtomicPtr::new(::std::ptr::null_mut())),
            dispatcher: super::default_dispatcher(),
        }
    }

    pub(crate) fn dead() -> ObjectMeta {
        ObjectMeta {
            alive: Arc::new(AtomicBool::new(false)),
            user_data: Arc::new(AtomicPtr::new(::std::ptr::null_mut())),
            dispatcher: super::default_dispatcher(),
        }
    }

    pub(crate) fn with_dispatcher<D: Dispatcher>(disp: D) -> ObjectMeta {
        ObjectMeta {
            alive: Arc::new(AtomicBool::new(true)),
            user_data: Arc::new(AtomicPtr::new(::std::ptr::null_mut())),
            dispatcher: Arc::new(Mutex::new(disp)),
        }
    }
}

#[derive(Clone)]
pub(crate) struct ResourceInner {
    pub(crate) id: u32,
    pub(crate) object: Object<ObjectMeta>,
    pub(crate) client: ClientInner,
    pub(crate) map: Arc<Mutex<ObjectMap<ObjectMeta>>>,
}

impl ResourceInner {
    pub(crate) fn from_id(
        id: u32,
        map: Arc<Mutex<ObjectMap<ObjectMeta>>>,
        client: ClientInner,
    ) -> Option<ResourceInner> {
        let me = map.lock().unwrap().find(id);
        me.map(|obj| ResourceInner {
            map,
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
                println!(
                    " -> {}@{}: {} {:?}",
                    I::NAME,
                    self.id,
                    self.object.events[msg.opcode as usize].name,
                    msg.args
                );
            }
            // TODO: figure our if this can fail and still be recoverable ?
            let _ = conn_lock.write_message(&msg).expect("Sending a message failed.");
            if destructor {
                self.object.meta.alive.store(false, Ordering::Release);
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

    pub(crate) fn set_user_data(&self, ptr: *mut ()) {
        self.object.meta.user_data.store(ptr, Ordering::Release)
    }

    pub(crate) fn get_user_data(&self) -> *mut () {
        self.object.meta.user_data.load(Ordering::Release)
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

    pub(crate) fn is_implemented_with<I: Interface, Impl>(&self) -> bool
    where
        Impl: Implementation<Resource<I>, I::Request> + 'static,
        I::Request: MessageGroup<Map = super::ResourceMap>,
    {
        self.object
            .meta
            .dispatcher
            .lock()
            .unwrap()
            .is::<super::ImplDispatcher<I, Impl>>()
    }
}

pub(crate) struct NewResourceInner {
    map: Arc<Mutex<ObjectMap<ObjectMeta>>>,
    client: ClientInner,
    id: u32,
}

impl NewResourceInner {
    pub(crate) fn from_id(
        id: u32,
        map: Arc<Mutex<ObjectMap<ObjectMeta>>>,
        client: ClientInner,
    ) -> Option<NewResourceInner> {
        if map.lock().unwrap().find(id).is_some() {
            Some(NewResourceInner { map, client, id })
        } else {
            None
        }
    }

    pub(crate) unsafe fn implement<I: Interface, Impl, Dest>(
        self,
        implementation: Impl,
        destructor: Option<Dest>,
        token: Option<&EventLoopInner>,
    ) -> ResourceInner
    where
        Impl: Implementation<Resource<I>, I::Request> + 'static,
        Dest: FnMut(Resource<I>, Box<Implementation<Resource<I>, I::Request>>) + 'static,
        I::Request: MessageGroup<Map = super::ResourceMap>,
    {
        let object = self.map.lock().unwrap().with(self.id, |obj| {
            obj.meta.dispatcher = super::make_dispatcher(implementation);
            obj.clone()
        });

        let object = match object {
            Ok(obj) => obj,
            Err(()) => {
                // We are tyring to implement a non-existent object
                // This is either a bug in the lib (a NewResource was created while it should not
                // have been possible) or an object was created and the client destroyed it
                // before it could be implemented.
                // Thus, we just create a dummy already-dead Resource
                Object::from_interface::<I>(1, ObjectMeta::dead())
            }
        };

        ResourceInner {
            map: self.map,
            client: self.client,
            id: self.id,
            object,
        }
    }
}
