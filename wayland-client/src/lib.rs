use wayland_backend::{
    client::{InvalidId, ObjectId, WaylandError},
    protocol::{Interface, Message},
};

mod cx;
mod event_queue;
pub mod globals;

pub mod backend {
    pub use wayland_backend::client::{
        Backend, Handle, InvalidId, NoWaylandLib, ObjectData, ObjectId, WaylandError,
    };
    pub use wayland_backend::protocol;
    pub use wayland_backend::smallvec;
}

pub use wayland_backend::protocol::WEnum;

pub use cx::{Connection, ConnectionHandle};
pub use event_queue::{Dispatch, EventQueue, QueueHandle, QueueProxyData};
//pub use event_queue::{event_stream, EventQueue, QueueHandle, Sink};

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

    fn version(&self) -> u32;

    fn data<D: Dispatch<Self> + 'static>(&self) -> Option<&<D as Dispatch<Self>>::UserData>;

    fn from_id(cx: &mut ConnectionHandle, id: ObjectId) -> Result<Self, InvalidId>;

    fn parse_event(
        cx: &mut ConnectionHandle,
        msg: Message<ObjectId>,
    ) -> Result<(Self, Self::Event), DispatchError>;

    fn write_request(
        &self,
        cx: &mut ConnectionHandle,
        req: Self::Request,
    ) -> Result<Message<ObjectId>, InvalidId>;
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
