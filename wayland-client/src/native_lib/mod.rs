mod display;
mod event_queue;
mod proxy;

pub(crate) use self::display::DisplayInner;
pub(crate) use self::event_queue::EventQueueInner;
pub(crate) use self::proxy::{NewProxyInner, ProxyInner};

pub enum ProxyMap {}

impl ProxyMap {
    pub fn get<I: Interface>(&mut self, id: u32) -> Option<Proxy<I>> {
        match *self {}
    }
    pub fn get_new<I: Interface>(&mut self, id: u32) -> Option<NewProxy<I>> {
        match *self {}
    }
}
