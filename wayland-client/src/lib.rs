//! wayland-client

#![warn(missing_docs, missing_debug_implementations)]
#![forbid(improper_ctypes, unsafe_op_in_unsafe_fn)]
#![cfg_attr(coverage, feature(no_coverage))]

use std::sync::Arc;
use wayland_backend::{
    client::{InvalidId, ObjectData, ObjectId, WaylandError},
    protocol::{Interface, Message},
};

mod conn;
mod event_queue;
pub mod globals;

/// Backend reexports
pub mod backend {
    pub use wayland_backend::client::{
        Backend, InvalidId, NoWaylandLib, ObjectData, ObjectId, ReadEventsGuard, WaylandError,
        WeakBackend,
    };
    pub use wayland_backend::protocol;
    pub use wayland_backend::smallvec;
}

pub use wayland_backend::protocol::WEnum;

pub use conn::{ConnectError, Connection};
pub use event_queue::{DelegateDispatch, Dispatch, EventQueue, QueueHandle, QueueProxyData};

/// Generated protocol definitions
///
/// This module is automatically generated from the `wayland.xml` protocol specification,
/// and contains the interface definitions for the core Wayland protocol.
#[allow(missing_docs)]
pub mod protocol {
    use self::__interfaces::*;
    use crate as wayland_client;
    pub mod __interfaces {
        wayland_scanner::generate_interfaces!("wayland.xml");
    }
    wayland_scanner::generate_client_code!("wayland.xml");
}

/// Trait representing a Wayland interface
pub trait Proxy: Sized {
    /// The event enum for this interface
    type Event;
    /// The request enum for this interface
    type Request;

    /// The interface description
    fn interface() -> &'static Interface;

    /// he ID of this object
    fn id(&self) -> ObjectId;

    /// The version of this object
    fn version(&self) -> u32;

    /// Access the user-data associated with this object
    fn data<U: Send + Sync + 'static>(&self) -> Option<&U>;

    /// Access the raw data associated with this object.
    ///
    /// For objects created using the scanner-generated methods, this will be an instance of the
    /// [QueueProxyData] type.
    fn object_data(&self) -> Option<&Arc<dyn ObjectData>>;

    /// Access the backend associated with this object
    fn backend(&self) -> &backend::WeakBackend;

    /// Create an object proxy from its ID
    ///
    /// Returns an error this the provided object ID does not correspond to
    /// the `Self` interface.
    ///
    /// **Note:** This method is mostly meant as an implementation detail to be
    /// used by code generated by wayland-scanner.
    fn from_id(conn: &Connection, id: ObjectId) -> Result<Self, InvalidId>;

    /// Send a request for this object.
    ///
    /// It is an error to use this function on requests that create objects; use
    /// [Proxy::send_constructor] for such requests.
    fn send_request(&self, req: Self::Request) -> Result<(), InvalidId>;

    /// Send a request for this object that creates another object.
    ///
    /// It is an error to use this function on requests that do not create objects; use
    /// [Proxy::send_request] for such requests.
    fn send_constructor<I: Self>(
        &self,
        req: Self::Request,
        data: Arc<dyn ObjectData>,
    ) -> Result<I, InvalidId>;

    /// Parse a event for this object
    ///
    /// **Note:** This method is mostly meant as an implementation detail to be
    /// used by code generated by wayland-scanner.
    fn parse_event(
        conn: &Connection,
        msg: Message<ObjectId>,
    ) -> Result<(Self, Self::Event), DispatchError>;

    /// Serialize a request for this object
    ///
    /// **Note:** This method is mostly meant as an implementation detail to be
    /// used by code generated by wayland-scanner.
    #[allow(clippy::type_complexity)]
    fn write_request(
        &self,
        conn: &Connection,
        req: Self::Request,
    ) -> Result<(Message<ObjectId>, Option<(&'static Interface, u32)>), InvalidId>;
}

/// Wayland dispatching error
#[derive(thiserror::Error, Debug)]
pub enum DispatchError {
    /// An invalid message was received
    #[error("Bad message for interface {interface} : {msg:?}")]
    BadMessage {
        /// The faulty message
        msg: Message<ObjectId>,
        /// The interface of the target object
        interface: &'static str,
    },
    /// The backend generated an error
    #[error("Backend error: {0}")]
    Backend(#[from] WaylandError),
}
