use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

use downcast::Downcast;

use wayland_commons::map::ObjectMap;
use wayland_commons::wire::Message;
use wayland_commons::MessageGroup;

use {Implementation, Interface, NewResource, Resource};

mod clients;
mod display;
mod event_loop;
mod globals;
mod resources;

pub(crate) use self::clients::ClientInner;
pub(crate) use self::display::DisplayInner;
pub(crate) use self::event_loop::{EventLoopInner, IdleSourceInner, SourceInner};
pub(crate) use self::globals::GlobalInner;
pub(crate) use self::resources::{NewResourceInner, ResourceInner};

pub struct ResourceMap {
    map: Arc<Mutex<ObjectMap<self::resources::ObjectMeta>>>,
    client: ClientInner,
}

impl ResourceMap {
    fn make(map: Arc<Mutex<ObjectMap<self::resources::ObjectMeta>>>, client: ClientInner) -> ResourceMap {
        ResourceMap { map, client }
    }
}

impl ResourceMap {
    pub fn get<I: Interface>(&mut self, id: u32) -> Option<Resource<I>> {
        ResourceInner::from_id(id, self.map.clone(), self.client.clone()).map(|object| {
            debug_assert!(I::NAME == "<anonymous>" || object.is_interface::<I>());
            Resource::wrap(object)
        })
    }
    pub fn get_new<I: Interface>(&mut self, id: u32) -> Option<NewResource<I>> {
        debug_assert!(
            self.map
                .lock()
                .unwrap()
                .find(id)
                .map(|obj| obj.is_interface::<I>())
                .unwrap_or(true)
        );
        NewResourceInner::from_id(id, self.map.clone(), self.client.clone())
            .map(|object| NewResource::wrap(object))
    }
}

pub(crate) trait Dispatcher: Downcast + Send {
    fn dispatch(&mut self, msg: Message, proxy: ResourceInner, map: &mut ResourceMap) -> Result<(), ()>;
}

impl_downcast!(Dispatcher);

pub(crate) struct ImplDispatcher<I: Interface, Impl: Implementation<Resource<I>, I::Request> + 'static> {
    _i: ::std::marker::PhantomData<&'static I>,
    implementation: Impl,
}

// This unsafe impl is "technically wrong", but enforced by the fact that
// the Impl will only ever be called from the same EventLoop, which is stuck
// on a single thread. The NewProxy::implement/implement_nonsend methods
// take care of ensuring that any non-Send impl is on the correct thread.
unsafe impl<I, Impl> Send for ImplDispatcher<I, Impl>
where
    I: Interface,
    Impl: Implementation<Resource<I>, I::Request> + 'static,
    I::Request: MessageGroup<Map = ResourceMap>,
{
}

impl<I, Impl> Dispatcher for ImplDispatcher<I, Impl>
where
    I: Interface,
    Impl: Implementation<Resource<I>, I::Request> + 'static,
    I::Request: MessageGroup<Map = ResourceMap>,
{
    fn dispatch(&mut self, msg: Message, resource: ResourceInner, map: &mut ResourceMap) -> Result<(), ()> {
        if ::std::env::var_os("WAYLAND_DEBUG").is_some() {
            println!(
                " <- {}@{}: {} {:?}",
                resource.object.interface,
                resource.id,
                resource.object.events[msg.opcode as usize].name,
                msg.args
            );
        }
        let message = I::Request::from_raw(msg, map)?;
        if message.is_destructor() {
            resource.object.meta.alive.store(false, Ordering::Release);
            if let Some(ref mut data) = *resource.client.data.lock().unwrap() {
                data.delete_id(resource.id);
            }
            self.implementation
                .receive(message, Resource::<I>::wrap(resource.clone()));
        } else {
            self.implementation
                .receive(message, Resource::<I>::wrap(resource));
        }
        Ok(())
    }
}

pub(crate) unsafe fn make_dispatcher<I, Impl>(implementation: Impl) -> Arc<Mutex<Dispatcher + Send>>
where
    I: Interface,
    Impl: Implementation<Resource<I>, I::Request> + 'static,
    I::Request: MessageGroup<Map = ResourceMap>,
{
    Arc::new(Mutex::new(ImplDispatcher {
        _i: ::std::marker::PhantomData,
        implementation,
    }))
}

pub(crate) fn default_dispatcher() -> Arc<Mutex<Dispatcher + Send>> {
    struct DefaultDisp;
    impl Dispatcher for DefaultDisp {
        fn dispatch(
            &mut self,
            _msg: Message,
            resource: ResourceInner,
            _map: &mut ResourceMap,
        ) -> Result<(), ()> {
            eprintln!(
                "[wayland-client] Received an event for unimplemented object {}@{}.",
                resource.object.interface, resource.id
            );
            Err(())
        }
    }

    Arc::new(Mutex::new(DefaultDisp))
}
