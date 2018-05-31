mod display;
mod event_queue;
mod proxy;

pub(crate) use self::display::DisplayInner;
pub(crate) use self::event_queue::EventQueueInner;
pub(crate) use self::proxy::{NewProxyInner, ProxyInner};
