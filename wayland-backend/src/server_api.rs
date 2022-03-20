use std::{
    ffi::CString,
    fmt,
    os::unix::{net::UnixStream, prelude::RawFd},
    sync::Arc,
};

use crate::protocol::{Interface, Message, ObjectInfo};
pub use crate::types::server::{Credentials, DisconnectReason, GlobalInfo, InitError, InvalidId};

use super::server_impl;

/// A trait representing your data associated to an object
///
/// You will only be given access to it as a `&` reference, so you
/// need to handle interior mutability by yourself.
///
/// The methods of this trait will be invoked internally every time a
/// new object is created to initialize its data.
pub trait ObjectData<D>: downcast_rs::DowncastSync {
    /// Dispatch a request for the associated object
    ///
    /// If the request has a NewId argument, the callback must return the object data
    /// for the newly created object
    fn request(
        self: Arc<Self>,
        handle: &mut Handle<D>,
        data: &mut D,
        client_id: ClientId,
        msg: Message<ObjectId>,
    ) -> Option<Arc<dyn ObjectData<D>>>;
    /// Notification that the object has been destroyed and is no longer active
    fn destroyed(&self, data: &mut D, client_id: ClientId, object_id: ObjectId);
    /// Helper for forwarding a Debug implementation of your `ObjectData` type
    ///
    /// By default will just print `ObjectData { ... }`
    fn debug(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ObjectData").finish_non_exhaustive()
    }
}

downcast_rs::impl_downcast!(sync ObjectData<D>);

#[cfg(not(tarpaulin_include))]
impl<D: 'static> std::fmt::Debug for dyn ObjectData<D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.debug(f)
    }
}

/// A trait representing the handling of new bound globals
pub trait GlobalHandler<D>: downcast_rs::DowncastSync {
    /// Check if given client is allowed to interact with given global
    ///
    /// If this function returns false, the client will not be notified of the existence
    /// of this global, and any attempt to bind it will result in a protocol error as if
    /// the global did not exist.
    ///
    /// Default implementation always return true.
    fn can_view(
        &self,
        _client_id: ClientId,
        _client_data: &Arc<dyn ClientData<D>>,
        _global_id: GlobalId,
    ) -> bool {
        true
    }
    /// A global has been bound
    ///
    /// Given client bound given global, creating given object.
    ///
    /// The method must return the object data for the newly created object.
    fn bind(
        self: Arc<Self>,
        handle: &mut Handle<D>,
        data: &mut D,
        client_id: ClientId,
        global_id: GlobalId,
        object_id: ObjectId,
    ) -> Arc<dyn ObjectData<D>>;
    /// Helper for forwarding a Debug implementation of your `GlobalHandler` type
    ///
    /// By default will just print `GlobalHandler { ... }`
    fn debug(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GlobalHandler").finish_non_exhaustive()
    }
}

#[cfg(not(tarpaulin_include))]
impl<D: 'static> std::fmt::Debug for dyn GlobalHandler<D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.debug(f)
    }
}

downcast_rs::impl_downcast!(sync GlobalHandler<D>);

/// A trait representing your data associated to a clientObjectData
pub trait ClientData<D>: downcast_rs::DowncastSync {
    /// Notification that a client was initialized
    fn initialized(&self, client_id: ClientId);

    /// Notification that a client is disconnected
    fn disconnected(&self, client_id: ClientId, reason: DisconnectReason);
    /// Helper for forwarding a Debug implementation of your `ClientData` type
    ///
    /// By default will just print `GlobalHandler { ... }`
    fn debug(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClientData").finish_non_exhaustive()
    }
}

#[cfg(not(tarpaulin_include))]
impl<D: 'static> std::fmt::Debug for dyn ClientData<D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.debug(f)
    }
}

downcast_rs::impl_downcast!(sync ClientData<D>);

/// An id of an object on a wayland server.
#[derive(Clone, PartialEq, Eq)]
pub struct ObjectId {
    pub(crate) id: server_impl::InnerObjectId,
}

impl ObjectId {
    /// Returns whether this object is a null object.
    pub fn is_null(&self) -> bool {
        self.id.is_null()
    }

    /// Returns the interface of this object.
    pub fn interface(&self) -> &'static Interface {
        self.id.interface()
    }

    /// Check if two object IDs are associated with the same client
    ///
    /// *Note:* This may spuriously return `false` if one (or both) of the objects to compare
    /// is no longer valid.
    pub fn same_client_as(&self, other: &ObjectId) -> bool {
        self.id.same_client_as(&other.id)
    }

    /// Return the protocol-level numerical ID of this object
    ///
    /// Protocol IDs are reused after object destruction, so this should not be used as a
    /// unique identifier,
    pub fn protocol_id(&self) -> u32 {
        self.id.protocol_id()
    }
}

#[cfg(not(tarpaulin_include))]
impl fmt::Display for ObjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.id.fmt(f)
    }
}

#[cfg(not(tarpaulin_include))]
impl fmt::Debug for ObjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.id.fmt(f)
    }
}

/// An id of a client connected to the server.
#[derive(Clone, PartialEq, Eq)]
pub struct ClientId {
    pub(crate) id: server_impl::InnerClientId,
}

#[cfg(not(tarpaulin_include))]
impl fmt::Debug for ClientId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.id.fmt(f)
    }
}

/// The ID of a global
#[derive(Clone, PartialEq, Eq)]
pub struct GlobalId {
    pub(crate) id: server_impl::InnerGlobalId,
}

#[cfg(not(tarpaulin_include))]
impl fmt::Debug for GlobalId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.id.fmt(f)
    }
}

/// Main handle of a backend to the Wayland protocol
///
/// This type hosts most of the protocol-related functionality of the backend, and is the
/// main entry point for manipulating Wayland objects. It can be retrieved both from
/// the backend via [`Backend::handle()`](super::Backend::handle), and is given to you as argument
/// in most event callbacks.
#[derive(Debug)]
pub struct Handle<D: 'static> {
    pub(crate) handle: server_impl::InnerHandle<D>,
}

impl<D> Handle<D> {
    /// Returns information about some object.
    #[inline]
    pub fn object_info(&self, id: ObjectId) -> Result<ObjectInfo, InvalidId> {
        self.handle.object_info(id.id)
    }

    /// Returns the id of the client which owns the object.
    #[inline]
    pub fn get_client(&self, id: ObjectId) -> Result<ClientId, InvalidId> {
        self.handle.get_client(id.id)
    }

    /// Returns the data associated with a client.
    #[inline]
    pub fn get_client_data(&self, id: ClientId) -> Result<Arc<dyn ClientData<D>>, InvalidId> {
        self.handle.get_client_data(id.id)
    }

    /// Retrive the [`Credentials`] of a client
    #[inline]
    pub fn get_client_credentials(&self, id: ClientId) -> Result<Credentials, InvalidId> {
        self.handle.get_client_credentials(id.id)
    }

    /// Returns an iterator over all clients connected to the server.
    #[inline]
    pub fn all_clients<'b>(&'b self) -> Box<dyn Iterator<Item = ClientId> + 'b> {
        self.handle.all_clients()
    }

    /// Returns an iterator over all objects owned by a client.
    #[inline]
    pub fn all_objects_for<'b>(
        &'b self,
        client_id: ClientId,
    ) -> Result<Box<dyn Iterator<Item = ObjectId> + 'b>, InvalidId> {
        self.handle.all_objects_for(client_id.id)
    }

    /// Retrieve the `ObjectId` for a wayland object given its protocol numerical ID
    #[inline]
    pub fn object_for_protocol_id(
        &self,
        client_id: ClientId,
        interface: &'static Interface,
        protocol_id: u32,
    ) -> Result<ObjectId, InvalidId> {
        self.handle.object_for_protocol_id(client_id.id, interface, protocol_id)
    }

    /// Create a new object for given client
    ///
    /// To ensure state coherence of the protocol, the created object should be immediately
    /// sent as a "New ID" argument in an event to the client.
    #[inline]
    pub fn create_object(
        &mut self,
        client_id: ClientId,
        interface: &'static Interface,
        version: u32,
        data: Arc<dyn ObjectData<D>>,
    ) -> Result<ObjectId, InvalidId> {
        self.handle.create_object(client_id.id, interface, version, data)
    }

    /// Returns an object id that represents a null object.
    #[inline]
    pub fn null_id(&mut self) -> ObjectId {
        self.handle.null_id()
    }

    /// Send an event to the client
    ///
    /// Returns an error if the sender ID of the provided message is no longer valid.
    ///
    /// **Panic:**
    ///
    /// Checks against the protocol specification are done, and this method will panic if they do
    /// not pass:
    ///
    /// - the message opcode must be valid for the sender interface
    /// - the argument list must match the prototype for the message associated with this opcode
    #[inline]
    pub fn send_event(&mut self, msg: Message<ObjectId>) -> Result<(), InvalidId> {
        self.handle.send_event(msg)
    }

    /// Returns the data associated with an object.
    #[inline]
    pub fn get_object_data(&self, id: ObjectId) -> Result<Arc<dyn ObjectData<D>>, InvalidId> {
        self.handle.get_object_data(id.id)
    }

    /// Sets the data associated with some object.
    #[inline]
    pub fn set_object_data(
        &mut self,
        id: ObjectId,
        data: Arc<dyn ObjectData<D>>,
    ) -> Result<(), InvalidId> {
        self.handle.set_object_data(id.id, data)
    }

    /// Posts an error on an object. This will also disconnect the client which created the object.
    #[inline]
    pub fn post_error(&mut self, object_id: ObjectId, error_code: u32, message: CString) {
        self.handle.post_error(object_id.id, error_code, message)
    }

    /// Kills the connection to a client.
    ///
    /// The disconnection reason determines the error message that is sent to the client (if any).
    #[inline]
    pub fn kill_client(&mut self, client_id: ClientId, reason: DisconnectReason) {
        self.handle.kill_client(client_id.id, reason)
    }

    /// Creates a global of the specified interface and version and then advertises it to clients.
    ///
    /// The clients which the global is advertised to is determined by the implementation of the [`GlobalHandler`].
    #[inline]
    pub fn create_global(
        &mut self,
        interface: &'static Interface,
        version: u32,
        handler: Arc<dyn GlobalHandler<D>>,
    ) -> GlobalId {
        GlobalId { id: self.handle.create_global(interface, version, handler) }
    }

    /// Disables a global object that is currently active.
    ///
    /// The global removal will be signaled to all currently connected clients. New clients will not know of the global,
    /// but the associated state and callbacks will not be freed. As such, clients that still try to bind the global
    /// afterwards (because they have not yet realized it was removed) will succeed.
    #[inline]
    pub fn disable_global(&mut self, id: GlobalId) {
        self.handle.disable_global(id.id)
    }

    /// Removes a global object and free its ressources.
    ///
    /// The global object will no longer be considered valid by the server, clients trying to bind it will be killed,
    /// and the global ID is freed for re-use.
    ///
    /// It is advised to first disable a global and wait some amount of time before removing it, to ensure all clients
    /// are correctly aware of its removal. Note that clients will generally not expect globals that represent a capability
    /// of the server to be removed, as opposed to globals representing peripherals (like `wl_output` or `wl_seat`).
    #[inline]
    pub fn remove_global(&mut self, id: GlobalId) {
        self.handle.remove_global(id.id)
    }

    /// Returns information about a global.
    #[inline]
    pub fn global_info(&self, id: GlobalId) -> Result<GlobalInfo, InvalidId> {
        self.handle.global_info(id.id)
    }

    /// Returns the handler which manages the visibility and notifies when a client has bound the global.
    #[inline]
    pub fn get_global_handler(&self, id: GlobalId) -> Result<Arc<dyn GlobalHandler<D>>, InvalidId> {
        self.handle.get_global_handler(id.id)
    }
}

/// A backend object that represents the state of a wayland server.
///
/// A backend is used to drive a wayland server by receiving requests, dispatching messages to the appropriate
/// handlers and flushes requests to be sent back to the client.
#[derive(Debug)]
pub struct Backend<D: 'static> {
    pub(crate) backend: server_impl::InnerBackend<D>,
}

impl<D> Backend<D> {
    /// Initialize a new Wayland backend
    #[inline]
    pub fn new() -> Result<Self, InitError> {
        Ok(Backend { backend: server_impl::InnerBackend::new()? })
    }

    /// Initializes a connection to a client.
    ///
    /// The `data` parameter contains data that will be associated with the client.
    #[inline]
    pub fn insert_client(
        &mut self,
        stream: UnixStream,
        data: Arc<dyn ClientData<D>>,
    ) -> std::io::Result<ClientId> {
        Ok(ClientId { id: self.backend.insert_client(stream, data)? })
    }

    /// Flushes pending events destined for a client.
    ///
    /// If no client is specified, all pending events are flushed to all clients.
    #[inline]
    pub fn flush(&mut self, client: Option<ClientId>) -> std::io::Result<()> {
        self.backend.flush(client)
    }

    /// Returns a handle which represents the server side state of the backend.
    ///
    /// The handle provides a variety of functionality, such as querying information about wayland objects,
    /// obtaining data associated with a client and it's objects, and creating globals.
    #[inline]
    pub fn handle(&mut self) -> &mut Handle<D> {
        self.backend.handle()
    }

    /// Returns the underlying file descriptor.
    ///
    /// The file descriptor may be monitored for activity with a polling mechanism such as epoll or kqueue.
    /// When it becomes readable, this means there are pending messages that would be dispatched if you call
    /// [`Backend::dispatch_all_clients`].
    ///
    /// The file descriptor should not be used for any other purpose than monitoring it.
    #[inline]
    pub fn poll_fd(&self) -> RawFd {
        self.backend.poll_fd()
    }

    /// Dispatches all pending messages from the specified client.
    ///
    /// This method will not block if there are no pending messages.
    ///
    /// The provided `data` will be provided to the handler of messages received from the client.
    ///
    /// For performance reasons, use of this function should be integrated with an event loop, monitoring the
    /// file descriptor associated with the client and only calling this method when messages are available.
    #[inline]
    pub fn dispatch_client(&mut self, data: &mut D, client_id: ClientId) -> std::io::Result<usize> {
        self.backend.dispatch_client(data, client_id.id)
    }

    /// Dispatches all pending messages from all clients.
    ///
    /// This method will not block if there are no pending messages.
    ///
    /// The provided `data` will be provided to the handler of messages received from the clients.
    ///
    /// For performance reasons, use of this function should be integrated with an event loop, monitoring the
    /// file descriptor retrieved by [`Backend::poll_fd`] and only calling this method when messages are
    /// available.
    #[inline]
    pub fn dispatch_all_clients(&mut self, data: &mut D) -> std::io::Result<usize> {
        self.backend.dispatch_all_clients(data)
    }
}

pub(crate) struct DumbObjectData;

impl<D> ObjectData<D> for DumbObjectData {
    fn request(
        self: Arc<Self>,
        _handle: &mut Handle<D>,
        _data: &mut D,
        _client_id: ClientId,
        _msg: Message<ObjectId>,
    ) -> Option<Arc<dyn ObjectData<D>>> {
        unreachable!()
    }

    fn destroyed(&self, _: &mut D, _client_id: ClientId, _object_id: ObjectId) {}
}
