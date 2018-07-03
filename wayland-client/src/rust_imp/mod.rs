use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

use downcast::Downcast;

use wayland_commons::map::ObjectMap;
use wayland_commons::wire::Message;
use wayland_commons::MessageGroup;

use {Implementation, Interface, NewProxy, Proxy};

mod connection;
mod display;
mod proxy;
mod queues;

pub(crate) use self::display::DisplayInner;
pub(crate) use self::proxy::{NewProxyInner, ProxyInner};
pub(crate) use self::queues::EventQueueInner;

pub struct ProxyMap {
    map: Arc<Mutex<ObjectMap<self::proxy::ObjectMeta>>>,
    connection: Arc<Mutex<self::connection::Connection>>,
}

impl ProxyMap {
    pub(crate) fn make(
        map: Arc<Mutex<ObjectMap<self::proxy::ObjectMeta>>>,
        connection: Arc<Mutex<self::connection::Connection>>,
    ) -> ProxyMap {
        ProxyMap { map, connection }
    }

    pub fn get<I: Interface>(&mut self, id: u32) -> Option<Proxy<I>> {
        ProxyInner::from_id(id, self.map.clone(), self.connection.clone()).map(|object| {
            debug_assert!(object.is_interface::<I>());
            Proxy::wrap(object)
        })
    }
    pub fn get_new<I: Interface>(&mut self, id: u32) -> Option<NewProxy<I>> {
        debug_assert!(
            self.map
                .lock()
                .unwrap()
                .find(id)
                .map(|obj| obj.is_interface::<I>())
                .unwrap_or(true)
        );
        NewProxyInner::from_id(id, self.map.clone(), self.connection.clone())
            .map(|object| NewProxy::wrap(object))
    }
}

pub(crate) trait Dispatcher: Downcast + Send {
    fn dispatch(&mut self, msg: Message, proxy: ProxyInner, map: &mut ProxyMap) -> Result<(), ()>;
}

impl_downcast!(Dispatcher);

pub(crate) struct ImplDispatcher<I: Interface, Impl: Implementation<Proxy<I>, I::Event> + 'static> {
    _i: ::std::marker::PhantomData<&'static I>,
    implementation: Impl,
}

// This unsafe impl is "technically wrong", but enforced by the fact that
// the Impl will only ever be called from the EventQueue, which is stuck
// on a single thread. The NewProxy::implement/implement_nonsend methods
// take care of ensuring that any non-Send impl is on the correct thread.
unsafe impl<I, Impl> Send for ImplDispatcher<I, Impl>
where
    I: Interface,
    Impl: Implementation<Proxy<I>, I::Event> + 'static,
    I::Event: MessageGroup<Map = ProxyMap>,
{
}

impl<I, Impl> Dispatcher for ImplDispatcher<I, Impl>
where
    I: Interface,
    Impl: Implementation<Proxy<I>, I::Event> + 'static,
    I::Event: MessageGroup<Map = ProxyMap>,
{
    fn dispatch(&mut self, msg: Message, proxy: ProxyInner, map: &mut ProxyMap) -> Result<(), ()> {
        let message = I::Event::from_raw(msg, map)?;
        if message.is_destructor() {
            proxy.object.meta.alive.store(false, Ordering::Release);
            proxy.map.lock().unwrap().kill(proxy.id);
            self.implementation
                .receive(message, Proxy::<I>::wrap(proxy.clone()));
        } else {
            self.implementation.receive(message, Proxy::<I>::wrap(proxy));
        }
        Ok(())
    }
}

pub(crate) unsafe fn make_dispatcher<I, Impl>(implementation: Impl) -> Arc<Mutex<Dispatcher + Send>>
where
    I: Interface,
    Impl: Implementation<Proxy<I>, I::Event> + 'static,
    I::Event: MessageGroup<Map = ProxyMap>,
{
    Arc::new(Mutex::new(ImplDispatcher {
        _i: ::std::marker::PhantomData,
        implementation,
    }))
}

pub(crate) fn default_dispatcher() -> Arc<Mutex<Dispatcher + Send>> {
    struct DefaultDisp;
    impl Dispatcher for DefaultDisp {
        fn dispatch(&mut self, msg: Message, proxy: ProxyInner, map: &mut ProxyMap) -> Result<(), ()> {
            eprintln!(
                "[wayland-client] Received an event for unimplemented object {}@{}.",
                proxy.object.interface, proxy.id
            );
            Err(())
        }
    }

    Arc::new(Mutex::new(DefaultDisp))
}
