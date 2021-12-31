#![warn(missing_debug_implementations)]

use wayland_backend::{
    protocol::{Interface, Message},
    server::{InvalidId, ObjectId},
};

mod client;
mod dispatch;
mod display;
mod global;
pub mod socket;

pub use client::Client;
pub use dispatch::{DataInit, DestructionNotify, Dispatch, New, ResourceData};
pub use display::{Display, DisplayHandle};
pub use global::GlobalDispatch;

pub mod backend {
    pub use wayland_backend::protocol;
    pub use wayland_backend::server::{
        Backend, ClientData, ClientId, DisconnectReason, GlobalHandler, GlobalId, Handle,
        InitError, InvalidId, ObjectData, ObjectId,
    };
    pub use wayland_backend::smallvec;
}

pub use wayland_backend::protocol::WEnum;

pub mod protocol {
    use self::__interfaces::*;
    use crate as wayland_server;
    pub mod __interfaces {
        wayland_scanner::generate_interfaces!("wayland.xml");
    }
    wayland_scanner::generate_server_code!("wayland.xml");
}

pub trait Resource: Sized {
    type Event;
    type Request;

    fn interface() -> &'static Interface;

    fn id(&self) -> ObjectId;

    fn version(&self) -> u32;

    fn data<U: 'static>(&self) -> Option<&U>;

    fn from_id<D>(dh: &mut DisplayHandle<D>, id: ObjectId) -> Result<Self, InvalidId>;

    fn parse_request<D>(
        dh: &mut DisplayHandle<D>,
        msg: Message<ObjectId>,
    ) -> Result<(Self, Self::Request), DispatchError>;

    fn write_event<D>(
        &self,
        dh: &mut DisplayHandle<D>,
        req: Self::Event,
    ) -> Result<Message<ObjectId>, InvalidId>;

    #[inline]
    fn post_error<D>(
        &self,
        dh: &mut DisplayHandle<D>,
        code: impl Into<u32>,
        error: impl Into<String>,
    ) {
        dh.post_error(self, code.into(), error.into())
    }

    #[doc(hidden)]
    fn __set_object_data(
        &mut self,
        odata: std::sync::Arc<dyn std::any::Any + Send + Sync + 'static>,
    );
}

#[derive(thiserror::Error, Debug)]
pub enum DispatchError {
    #[error("Bad message for interface {interface} : {msg:?}")]
    BadMessage { msg: Message<ObjectId>, interface: &'static str },
    #[error("Unexpected interface {interface} for message {msg:?}")]
    NoHandler { msg: Message<ObjectId>, interface: &'static str },
}
