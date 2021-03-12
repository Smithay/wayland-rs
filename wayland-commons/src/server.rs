use std::{
    ffi::CString,
    os::unix::{io::RawFd, net::UnixStream},
    sync::Arc,
};

use downcast_rs::DowncastSync;

use crate::client;

use super::{Argument, Interface, ObjectInfo};

pub struct GlobalInfo {
    pub interface: &'static Interface,
    pub version: u32,
    pub disabled: bool,
}

/// A trait representing your data associated to an object
///
/// You will only be given access to it as a `&` reference, so you
/// need to handle interior mutability by yourself.
///
/// The methods of this trait will be invoked internally every time a
/// new object is created to initialize its data.
pub trait ObjectData<B: ServerBackend>: downcast_rs::DowncastSync {
    /// Create a new object data from the parent data
    fn make_child(self: Arc<Self>, child_info: &ObjectInfo) -> Arc<dyn ObjectData<B>>;
    /// Dispatch a request for the associated object
    fn request(
        &self,
        handle: &mut B::Handle,
        client_id: B::ClientId,
        object_id: B::ObjectId,
        opcode: u16,
        arguments: &[Argument<B::ObjectId>],
    );
    /// Notification that the object has been destroyed and is no longer active
    fn destroyed(&self, client_id: B::ClientId, object_id: B::ObjectId);
}

downcast_rs::impl_downcast!(sync ObjectData<B> where B: ServerBackend);

/// A trait representing the handling of new bound globals
pub trait GlobalHandler<B: ServerBackend>: downcast_rs::DowncastSync {
    /// Check if given client is allowed to interact with given global
    ///
    /// If this function returns false, the client will not be notified of the existence
    /// of this global, and any attempt to bind it will result in a protocol error as if
    /// the global did not exist.
    ///
    /// Default implementation always return true.
    fn can_view(&self, client_id: B::ClientId, global_id: B::GlobalId) -> bool {
        true
    }
    /// Create the ObjectData for a future bound global
    fn make_data(self: Arc<Self>, info: &ObjectInfo) -> Arc<dyn ObjectData<B>>;
    /// A global has been bound
    ///
    /// Given client bound given global, creating given object.
    fn bind(
        &self,
        handle: &mut B::Handle,
        client_id: B::ClientId,
        global_id: B::GlobalId,
        object_id: B::ObjectId,
    );
}

downcast_rs::impl_downcast!(sync GlobalHandler<B> where B: ServerBackend);

/// A trait representing your data associated to a clientObjectData
pub trait ClientData<B: ServerBackend>: downcast_rs::DowncastSync {
    /// Notification that a client was initialized
    fn initialized(&self, client_id: B::ClientId);

    /// Notification that a client is disconnected
    fn disconnected(&self, client_id: B::ClientId, reason: DisconnectReason);
}

downcast_rs::impl_downcast!(sync ClientData<B> where B: ServerBackend);

pub trait ObjectId: Clone + Send + std::fmt::Debug {
    fn is_null(&self) -> bool;
}

pub trait ServerBackend: Sized {
    type ObjectId: ObjectId;
    type ClientId: Clone + Send + std::fmt::Debug;
    type GlobalId: Clone + Send + std::fmt::Debug;
    type Handle: BackendHandle<Self>;
    type InitError: std::error::Error;

    /// Initialize the backend
    fn new() -> Result<Self, Self::InitError>;

    /// Initialize a client on a connected unix socket
    fn insert_client(
        &mut self,
        stream: UnixStream,
        data: Arc<dyn ClientData<Self>>,
    ) -> std::io::Result<Self::ClientId>;

    /// Flush the internal outgoing buffers to clients
    ///
    /// If `client` is some, only the associated client is flushed, otherwise
    /// all clients are flushed.
    fn flush(&mut self, client: Option<Self::ClientId>) -> std::io::Result<()>;

    /// Access the handle for protocol interaction with this backend.
    fn handle(&mut self) -> &mut Self::Handle;
}

/// Trait representing a server backends that internally handles the polling of clients
///
/// This backend presents the set of clients as a whole, and gives you a single file
/// descriptor for monitoring it. It maintains an internal epoll-like instance and
/// client FDs are added to it when you invoke `insert_client`.
pub trait CommonPollBackend: ServerBackend {
    /// Get an FD for monitoring using epoll or equivalent
    fn poll_fd(&self) -> RawFd;

    /// Read and dispatch incoming requests
    ///
    /// This function never blocks. If new events are available, they are read
    /// from the socket and the `event()` method of the `ObjectData` associated
    /// to their target object is invoked, sequentially.
    fn dispatch_events(&mut self) -> std::io::Result<usize>;
}

/// Trait representing a server backends that keeps clients independent
///
/// This backend does not poll clients for you, and instead requires you to
/// maintain the polling logic yourself and dispatch clients one by one manually.
///
/// It can be used for setups when you want to implement priority mechanisms between
/// clients, for example.
pub trait IndependentBackend: ServerBackend {
    /// Read and dispatch incoming requests for a single client
    ///
    /// This function never blocks. If new events are available, they are read
    /// from the socket and the `event()` method of the `ObjectData` associated
    /// to their target object is invoked, sequentially.
    fn dispatch_events_for(
        &mut self,
        client_id: <Self as ServerBackend>::ClientId,
    ) -> std::io::Result<usize>;
}

pub trait BackendHandle<B: ServerBackend> {
    /// Get the object info associated to given object
    fn object_info(&self, id: B::ObjectId) -> Result<ObjectInfo, InvalidId>;

    /// Retrieve the client ID associated with a given object
    fn get_client(&self, id: B::ObjectId) -> Result<B::ClientId, InvalidId>;

    /// Access the `ObjectData` associated with a given object id
    fn get_client_data(&self, id: B::ClientId) -> Result<Arc<dyn ClientData<B>>, InvalidId>;

    /// An iterator over all known clients
    fn all_clients<'a>(&'a self) -> Box<dyn Iterator<Item = B::ClientId> + 'a>;

    /// An iterator over all objects of a client
    fn all_objects_for<'a>(
        &'a self,
        client_id: B::ClientId,
    ) -> Result<Box<dyn Iterator<Item = B::ObjectId> + 'a>, InvalidId>;

    /// Create a new object for given client
    ///
    /// The created object should immediately be sent to the client with the appropriate
    /// constructor event.
    fn create_object(
        &mut self,
        client: B::ClientId,
        interface: &'static Interface,
        version: u32,
        data: Arc<dyn ObjectData<B>>,
    ) -> Result<B::ObjectId, InvalidId>;

    /// Create a null id, to be used with `send_request()` or `send_constructor()`
    ///
    /// To be used to represent an optional object that is absent.
    fn null_id(&mut self) -> B::ObjectId;

    /// Send an event to a client
    fn send_event(
        &mut self,
        object_id: B::ObjectId,
        opcode: u16,
        args: &[Argument<B::ObjectId>],
    ) -> Result<(), InvalidId>;

    /// Access the `ObjectData` associated with a given object id
    fn get_object_data(&self, id: B::ObjectId) -> Result<Arc<dyn ObjectData<B>>, InvalidId>;

    /// Trigger a protocol error on given object
    ///
    /// The associated client will be disconnected after the error has been sent
    /// to it.
    fn post_error(&mut self, object_id: B::ObjectId, error_code: u32, message: CString);

    /// Disconnect a client
    ///
    /// The connection of this client will be terminated without sending it anything. When applicable,
    /// `post_error()` should generally be preferred.
    fn kill_client(&mut self, client_id: B::ClientId, reason: DisconnectReason);

    fn create_global(
        &mut self,
        interface: &'static Interface,
        version: u32,
        handler: Arc<dyn GlobalHandler<B>>,
    ) -> B::GlobalId;

    fn disable_global(&mut self, id: B::GlobalId);

    fn remove_global(&mut self, id: B::GlobalId);

    fn global_info(&self, id: B::GlobalId) -> Result<GlobalInfo, InvalidId>;

    fn get_global_handler(&self, id: B::GlobalId) -> Result<Arc<dyn GlobalHandler<B>>, InvalidId>;
}

/// An error type representing the failure to load libwayland
#[derive(Debug)]
pub struct NoWaylandLib;

impl std::error::Error for NoWaylandLib {}

impl std::fmt::Display for NoWaylandLib {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> Result<(), ::std::fmt::Error> {
        f.write_str("could not load libwayland-server.so")
    }
}

/// An error generated when trying to act on an invalid `ObjectId`.
#[derive(Clone, Debug)]
pub struct InvalidId;

impl std::error::Error for InvalidId {}

impl std::fmt::Display for InvalidId {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> Result<(), ::std::fmt::Error> {
        write!(f, "Invalid Id")
    }
}

pub enum DisconnectReason {
    ConnectionClosed,
    ProtocolError(super::ProtocolError),
}
