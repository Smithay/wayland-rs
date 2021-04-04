use std::{
    os::unix::{io::RawFd, net::UnixStream},
    sync::Arc,
};

use super::{Argument, Interface, Message, ObjectInfo};

/// A trait representing your data associated to an object
///
/// You will only be given access to it as a `&` reference, so you
/// need to handle interior mutability by yourself.
///
/// The methods of this trait will be invoked internally every time a
/// new object is created to initialize its data.
pub trait ObjectData<B: ClientBackend>: downcast_rs::DowncastSync {
    /// Create a new object data from the parent data
    fn make_child(self: Arc<Self>, child_info: &ObjectInfo) -> Arc<dyn ObjectData<B>>;
    /// Dispatch an event for the associated object
    fn event(&self, handle: &mut B::Handle, msg: Message<B::ObjectId>);
    /// Notification that the object has been destroyed and is no longer active
    fn destroyed(&self, object_id: B::ObjectId);
}

downcast_rs::impl_downcast!(sync ObjectData<B> where B: ClientBackend);

pub trait ClientBackend: Sized {
    type ObjectId: ObjectId;
    type Handle: BackendHandle<Self>;
    type InitError: std::error::Error;

    /// Initialize the wayland state on a connected unix socket
    fn connect(stream: UnixStream) -> Result<Self, Self::InitError>;

    /// Get the connection FD for monitoring using epoll or equivalent
    fn connection_fd(&self) -> RawFd;

    /// Flush the internal outgoing buffers to the server
    fn flush(&mut self) -> Result<(), WaylandError>;

    /// Try to read and dispatch incoming events
    ///
    /// This function never blocks. If new events are available, they are read
    /// from the socket and the `event()` method of the `ObjectData` associated
    /// to their target object is invoked, sequentially.
    fn dispatch_events(&mut self) -> Result<usize, WaylandError>;

    /// Access the handle for protocol interaction with this backend.
    fn handle(&mut self) -> &mut Self::Handle;
}

pub trait ObjectId: Clone + Send + std::fmt::Debug {
    fn is_null(&self) -> bool;
}

pub trait BackendHandle<B: ClientBackend> {
    /// Get the `wl_display` id
    fn display_id(&self) -> B::ObjectId;

    /// Retrieve the last error that occured if the connection is in an error state
    fn last_error(&self) -> Option<WaylandError>;

    /// Get the object info associated to given object
    fn info(&self, id: B::ObjectId) -> Result<ObjectInfo, InvalidId>;

    /// Create a placeholder id, to be used with `send_request()`.
    ///
    /// Optionnaly the expected interface and version of the to-be-created object can be provided.
    /// If they are provided, they will be checked against the ones derived from the protocol
    /// specification by a `debug_assert!()`.
    fn placeholder_id(&mut self, spec: Option<(&'static Interface, u32)>) -> B::ObjectId;

    /// Create a null id, to be used with `send_request()` or `send_constructor()`
    ///
    /// To be used to represent an optional object that is absent.
    fn null_id(&mut self) -> B::ObjectId;

    /// Send a request possibly creating a new object
    ///
    /// The id of the newly created object is returned, or a `null_id()` if the request does not
    /// create an object. The provided arguments must contain at most one `NewId`, filled with a
    /// placeholder id created from `placeholder_id()`.
    ///
    /// If the interface and version of the created object cannot be derived from the protocol
    /// specification (notable example being `wl_registry.bind`), then they must have been given to
    /// `placeholder_id()`.
    ///
    /// The optional `data` provided will be used as `ObjectData` for the created object. If `None`,
    /// the `ObjectData` will instead be used by invoking `ObjectData::make_chikd()` on the parent
    /// data. If the parent object is the `wl_display`, then some `ObjectData` *must* be provided.
    /// Failing to do so will cause a panic.
    fn send_request(
        &mut self,
        msg: Message<B::ObjectId>,
        data: Option<Arc<dyn ObjectData<B>>>,
    ) -> Result<B::ObjectId, InvalidId>;

    /// Access the `ObjectData` associated with a given object id
    fn get_data(&self, id: B::ObjectId) -> Result<Arc<dyn ObjectData<B>>, InvalidId>;
}

/// An error type representing the failure to load libwayland
#[derive(Debug)]
pub struct NoWaylandLib;

impl std::error::Error for NoWaylandLib {}

impl std::fmt::Display for NoWaylandLib {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> Result<(), ::std::fmt::Error> {
        f.write_str("could not load libwayland-client.so")
    }
}

/// An error that can occur when using a Wayland connection
#[derive(Debug)]
pub enum WaylandError {
    /// The connection encountered an IO error
    Io(std::io::Error),
    /// The connection encountered a protocol error
    Protocol(super::ProtocolError),
}

impl std::error::Error for WaylandError {
    fn cause(&self) -> Option<&dyn std::error::Error> {
        match self {
            WaylandError::Io(e) => Some(e),
            WaylandError::Protocol(e) => Some(e),
        }
    }
}

impl std::fmt::Display for WaylandError {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> Result<(), ::std::fmt::Error> {
        match self {
            WaylandError::Io(e) => write!(f, "Io error: {}", e),
            WaylandError::Protocol(e) => std::fmt::Display::fmt(e, f),
        }
    }
}

impl Clone for WaylandError {
    fn clone(&self) -> WaylandError {
        match self {
            WaylandError::Protocol(e) => WaylandError::Protocol(e.clone()),
            WaylandError::Io(e) => {
                if let Some(code) = e.raw_os_error() {
                    WaylandError::Io(std::io::Error::from_raw_os_error(code))
                } else {
                    WaylandError::Io(std::io::Error::new(e.kind(), ""))
                }
            }
        }
    }
}

impl From<super::ProtocolError> for WaylandError {
    fn from(err: super::ProtocolError) -> WaylandError {
        WaylandError::Protocol(err)
    }
}

impl From<std::io::Error> for WaylandError {
    fn from(err: std::io::Error) -> WaylandError {
        WaylandError::Io(err)
    }
}

/// An error generated when trying to act on an invalid `ObjectId`.
#[derive(Clone, Debug)]
pub struct InvalidId;

impl std::error::Error for InvalidId {}

impl std::fmt::Display for InvalidId {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> Result<(), ::std::fmt::Error> {
        write!(f, "Invalid ObjectId")
    }
}
