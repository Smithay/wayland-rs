use std::ffi::OsString;
use std::io;
use std::sync::Arc;

use protocol::wl_display::WlDisplay;

use {ConnectError, Implementation, Interface, Proxy};

mod connection;
mod queues;

pub(crate) struct DisplayInner {}

impl DisplayInner {
    pub fn connect_to_name(
        name: Option<OsString>,
    ) -> Result<(Arc<DisplayInner>, EventQueueInner), ConnectError> {
        unimplemented!()
    }

    pub(crate) fn flush(&self) -> io::Result<i32> {
        unimplemented!()
    }

    pub(crate) fn create_event_queue(me: &Arc<DisplayInner>) -> EventQueueInner {
        unimplemented!()
    }

    pub(crate) fn get_proxy(&self) -> &Proxy<WlDisplay> {
        unimplemented!()
    }
}

pub(crate) struct EventQueueInner {}

impl EventQueueInner {
    pub fn dispatch(&mut self) -> io::Result<u32> {
        unimplemented!()
    }

    pub fn dispatch_pending(&mut self) -> io::Result<u32> {
        unimplemented!()
    }

    pub fn sync_roundtrip(&mut self) -> io::Result<i32> {
        unimplemented!()
    }

    pub(crate) fn prepare_read(&self) -> Result<(), ()> {
        unimplemented!()
    }

    pub(crate) fn read_events(&self) -> io::Result<i32> {
        unimplemented!()
    }

    pub(crate) fn cancel_read(&self) {
        unimplemented!()
    }
}

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
