use wayland_backend::{
    client::{InvalidId, ObjectId},
    protocol::{Interface, Message},
};

mod cx;
pub mod proxy_internals;

pub mod backend {
    pub use wayland_backend::client::{
        Backend, Handle, InvalidId, NoWaylandLib, ObjectData, ObjectId, WaylandError,
    };
    pub use wayland_backend::protocol;
    pub use wayland_backend::smallvec;
}

pub use cx::{Connection, ConnectionHandle};

pub mod protocol {
    use crate as wayland_client;
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

#[derive(thiserror::Error, Debug)]
pub enum DispatchError {
    #[error("Bad message for interface {interface} : {msg:?}")]
    BadMessage { msg: Message<ObjectId>, interface: &'static Interface },
    #[error("Unexpected interface {interface} for message {msg:?}")]
    NoHandler { msg: Message<ObjectId>, interface: &'static Interface },
}
