use {Implementation, Interface, Proxy};

mod connection;
mod display;
mod queues;

pub(crate) use self::display::DisplayInner;
pub(crate) use self::queues::EventQueueInner;

#[derive(Clone)]
pub(crate) struct ProxyInner {}

impl ProxyInner {
    pub(crate) fn is_alive(&self) -> bool {
        unimplemented!()
    }

    pub fn version(&self) -> u32 {
        unimplemented!()
    }

    pub fn set_user_data(&self, ptr: *mut ()) {
        unimplemented!()
    }

    pub fn get_user_data(&self) -> *mut () {
        unimplemented!()
    }

    pub(crate) fn send<I: Interface>(&self, msg: I::Request) {
        unimplemented!()
    }

    pub(crate) fn equals(&self, other: &ProxyInner) -> bool {
        unimplemented!()
    }

    pub(crate) fn make_wrapper(&self, queue: &EventQueueInner) -> Result<ProxyInner, ()> {
        unimplemented!()
    }

    pub(crate) fn is_implemented_with<I: Interface, Impl>(&self) -> bool
    where
        Impl: Implementation<Proxy<I>, I::Event> + 'static,
    {
        unimplemented!()
    }

    pub(crate) fn child<I: Interface>(&self) -> NewProxyInner {
        unimplemented!()
    }
}

pub(crate) struct NewProxyInner {}

impl NewProxyInner {
    pub(crate) unsafe fn implement<I: Interface, Impl>(self, implementation: Impl) -> ProxyInner
    where
        Impl: Implementation<Proxy<I>, I::Event> + 'static,
    {
        unimplemented!()
    }
}
