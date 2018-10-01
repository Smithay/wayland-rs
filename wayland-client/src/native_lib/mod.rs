mod display;
mod event_queue;
mod proxy;

pub(crate) use self::display::DisplayInner;
pub(crate) use self::event_queue::EventQueueInner;
pub(crate) use self::proxy::{NewProxyInner, ProxyInner};

use {Interface, NewProxy, Proxy};

/// This type only exists for type-level compatibility
/// with the rust implementation.
///
/// It is an empty enum that cannot be instantiated
pub enum ProxyMap {}

impl ProxyMap {
    /// Unusable method only existing for type-level compatibility
    pub fn get<I: Interface>(&mut self, _: u32) -> Option<Proxy<I>> {
        match *self {}
    }

    /// Unusable method only existing for type-level compatibility
    pub fn get_new<I: Interface>(&mut self, _: u32) -> Option<NewProxy<I>> {
        match *self {}
    }
}
