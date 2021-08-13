use wayland_backend::{
    client::{InvalidId, ObjectId, WaylandError},
    protocol::{Interface, Message},
};

mod cx;
mod event_queue;
pub mod proxy_internals;

pub mod backend {
    pub use wayland_backend::client::{
        Backend, Handle, InvalidId, NoWaylandLib, ObjectData, ObjectId, WaylandError,
    };
    pub use wayland_backend::protocol;
    pub use wayland_backend::smallvec;
}

pub use wayland_backend::protocol::WEnum;

pub use cx::{Connection, ConnectionHandle};
pub use event_queue::{event_stream, EventQueue, QueueHandle, Sink};

pub mod protocol {
    use self::__interfaces::*;
    use crate as wayland_client;
    pub mod __interfaces {
        wayland_scanner::generate_interfaces!("wayland.xml");
    }
    wayland_scanner::generate_client_code!("wayland.xml");
}

pub trait Proxy: Sized {
    type Event;
    type Request;

    fn interface() -> &'static Interface;

    fn id(&self) -> ObjectId;

    fn data(&self) -> Option<&std::sync::Arc<proxy_internals::ProxyData>>;

    fn from_id(cx: &mut ConnectionHandle, id: ObjectId) -> Result<Self, InvalidId>;

    fn parse_event(
        cx: &mut ConnectionHandle,
        msg: Message<ObjectId>,
    ) -> Result<(Self, Self::Event), Message<ObjectId>>;

    fn write_request(
        &self,
        cx: &mut ConnectionHandle,
        req: Self::Request,
    ) -> Result<Message<ObjectId>, InvalidId>;

    #[inline]
    fn init_user_data<T: 'static + Send + Sync>(
        &self,
        f: impl FnOnce() -> T,
    ) -> Result<(), InvalidId> {
        self.data().ok_or(InvalidId)?.init_user_data(|| Box::new(f()));
        Ok(())
    }

    #[inline]
    fn get_user_data<T: 'static>(&self) -> Option<&T> {
        self.data()?.get_user_data()?.downcast_ref()
    }
}

pub trait FromEvent {
    type Out;

    fn from_event(
        cx: &mut ConnectionHandle,
        msg: Message<ObjectId>,
    ) -> Result<Self::Out, DispatchError>;
}

impl<I: Proxy> FromEvent for I {
    type Out = (I, I::Event);

    fn from_event(
        cx: &mut ConnectionHandle,
        msg: Message<ObjectId>,
    ) -> Result<Self::Out, DispatchError> {
        let sender_iface = msg.sender_id.interface();
        if crate::backend::protocol::same_interface(sender_iface, I::interface()) {
            I::parse_event(cx, msg)
                .map_err(|msg| DispatchError::BadMessage { msg, interface: sender_iface })
        } else {
            Err(DispatchError::NoHandler { msg, interface: sender_iface })
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum DispatchError {
    #[error("Bad message for interface {interface} : {msg:?}")]
    BadMessage { msg: Message<ObjectId>, interface: &'static Interface },
    #[error("Unexpected interface {interface} for message {msg:?}")]
    NoHandler { msg: Message<ObjectId>, interface: &'static Interface },
    #[error("Backend error: {0}")]
    Backend(#[from] WaylandError),
}
