use std::{
    fmt,
    os::unix::{io::RawFd, net::UnixStream},
    sync::{Arc, Mutex},
};

use crate::protocol::{Interface, Message, ObjectInfo};

use super::client_impl;

pub use crate::types::client::{InvalidId, NoWaylandLib, WaylandError};

/// A trait representing your data associated to an object
///
/// You will only be given access to it as a `&` reference, so you
/// need to handle interior mutability by yourself.
///
/// The methods of this trait will be invoked internally every time a
/// new object is created to initialize its data.
pub trait ObjectData: downcast_rs::DowncastSync {
    /// Dispatch an event for the associated object
    ///
    /// If the event has a NewId argument, the callback must return the object data
    /// for the newly created object
    fn event(
        self: Arc<Self>,
        handle: &mut Handle,
        msg: Message<ObjectId>,
    ) -> Option<Arc<dyn ObjectData>>;
    /// Notification that the object has been destroyed and is no longer active
    fn destroyed(&self, object_id: ObjectId);
    /// Helper for forwarding a Debug implementation of your `ObjectData` type
    ///
    /// By default will just print `ObjectData { ... }`
    #[cfg_attr(coverage, no_coverage)]
    fn debug(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ObjectData").finish_non_exhaustive()
    }
}

impl std::fmt::Debug for dyn ObjectData {
    #[cfg_attr(coverage, no_coverage)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.debug(f)
    }
}

downcast_rs::impl_downcast!(sync ObjectData);

/// An ID representing a Wayland object
#[derive(Clone, PartialEq, Eq)]
pub struct ObjectId {
    pub(crate) id: client_impl::InnerObjectId,
}

impl fmt::Display for ObjectId {
    #[cfg_attr(coverage, no_coverage)]
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.id.fmt(f)
    }
}

impl fmt::Debug for ObjectId {
    #[cfg_attr(coverage, no_coverage)]
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.id.fmt(f)
    }
}

impl ObjectId {
    /// Check if this is the null ID
    #[inline]
    pub fn is_null(&self) -> bool {
        self.id.is_null()
    }

    /// Interface of the represented object
    #[inline]
    pub fn interface(&self) -> &'static Interface {
        self.id.interface()
    }

    /// Return the protocol-level numerical ID of this object
    ///
    /// Protocol IDs are reused after object destruction, so this should not be used as a
    /// unique identifier,
    #[inline]
    pub fn protocol_id(&self) -> u32 {
        self.id.protocol_id()
    }
}

/// Main handle of a backend to the Wayland protocol
///
/// This type hosts most of the protocol-related functionality of the backend, and is the
/// main entry point for manipulating Wayland objects. It can be retrieved both from
/// the backend via [`Backend::handle()`](Backend::handle), and is given to you as argument
/// in most event callbacks.
#[derive(Debug)]
pub struct Handle {
    pub(crate) handle: client_impl::InnerHandle,
}

/// A pure rust implementation of a Wayland client backend
///
/// This type hosts the plumbing functionalities for interacting with the wayland protocol,
/// and most of the protocol-level interactions are made through the [`Handle`] type, accessed
/// via the [`handle()`](Backend::handle) method.
#[derive(Debug)]
pub struct Backend {
    pub(crate) backend: client_impl::InnerBackend,
}

impl Backend {
    /// Try to initialize a Wayland backend on the provided unix stream
    ///
    /// The provided stream should correspond to an already established unix connection with
    /// the Wayland server. On this rust backend, this method never fails.
    pub fn connect(stream: UnixStream) -> Result<Self, NoWaylandLib> {
        client_impl::InnerBackend::connect(stream).map(|backend| Backend { backend })
    }

    /// Flush all pending outgoing requests to the server
    pub fn flush(&mut self) -> Result<(), WaylandError> {
        self.backend.flush()
    }

    /// Read events from the wayland socket if available, and invoke the associated callbacks
    ///
    /// This function will never block, and returns an I/O `WouldBlock` error if no event is available
    /// to read.
    ///
    /// **Note:** this function should only be used if you know that you are the only thread
    /// reading events from the wayland socket. If this may not be the case, see [`ReadEventsGuard`]
    pub fn dispatch_events(&mut self) -> Result<usize, WaylandError> {
        self.backend.dispatch_events()
    }

    /// Access the [`Handle`] associated with this backend
    pub fn handle(&mut self) -> &mut Handle {
        self.backend.handle()
    }
}

/// Guard for synchronizing event reading across multiple threads
///
/// If multiple threads need to read events from the Wayland socket concurrently,
/// it is necessary to synchronize their access. Failing to do so may cause some of the
/// threads to not be notified of new events, and sleep much longer than appropriate.
///
/// To correctly synchronize access, this type should be used. The guard is created using
/// the [`try_new()`](ReadEventsGuard::try_new) method. And the event reading is triggered by consuming
/// the guard using the [`read()`](ReadEventsGuard::read) method.
///
/// If you plan to poll the Wayland socket for readiness, the file descriptor can be retrieved via
/// the [`connection_fd`](ReadEventsGuard::connection_fd) method. Note that for the synchronization to
/// correctly occur, you must *always* create the `ReadEventsGuard` *before* polling the socket.
#[derive(Debug)]
pub struct ReadEventsGuard {
    pub(crate) guard: client_impl::InnerReadEventsGuard,
}

impl ReadEventsGuard {
    /// Create a new reading guard
    ///
    /// This call will not block, but event callbacks may be invoked in the process
    /// of preparing the guard.
    #[inline]
    pub fn try_new(backend: Arc<Mutex<Backend>>) -> Result<Self, WaylandError> {
        client_impl::InnerReadEventsGuard::try_new(backend).map(|guard| ReadEventsGuard { guard })
    }

    /// Access the Wayland socket FD for polling
    #[inline]
    pub fn connection_fd(&self) -> RawFd {
        self.guard.connection_fd()
    }

    /// Attempt to read events from the Wayland socket
    ///
    /// If multiple threads have a live reading guard, this method will block until all of them
    /// are either dropped or have their `read()` method invoked, at which point on of the threads
    /// will read events from the socket and invoke the callbacks for the received events. All
    /// threads will then resume their execution.
    ///
    /// This returns the number of dispatched events, or `0` if an other thread handled the dispatching.
    /// If no events are available to read from the socket, this returns a `WouldBlock` IO error.
    #[inline]
    pub fn read(self) -> Result<usize, WaylandError> {
        self.guard.read()
    }
}

impl Handle {
    /// Get the object ID for the `wl_display`
    #[inline]
    pub fn display_id(&self) -> ObjectId {
        self.handle.display_id()
    }

    /// Get the last error that occurred on this backend
    ///
    /// If this returns an error, your Wayland connection is already dead.
    #[inline]
    pub fn last_error(&self) -> Option<WaylandError> {
        self.handle.last_error()
    }

    /// Get the detailed information about a wayland object
    ///
    /// Returns an error if the provided object ID is no longer valid.
    #[inline]
    pub fn info(&self, id: ObjectId) -> Result<ObjectInfo, InvalidId> {
        self.handle.info(id)
    }

    /// Create a null object ID
    ///
    /// This object ID is always invalid, and can be used as placeholder.
    #[inline]
    pub fn null_id(&mut self) -> ObjectId {
        self.handle.null_id()
    }

    /// Create a placeholder ID for object creation
    ///
    /// This ID needs to be created beforehand and given as argument to a request creating a
    /// new object ID. A specification must be specified if the interface and version cannot
    /// be inferred from the protocol (for example object creation from the `wl_registry`).
    ///
    /// If a specification is provided it'll be checked against what can be deduced from the
    /// protocol specification, and [`send_request`](Handle::send_request) will panic if they
    /// do not match.
    #[inline]
    pub fn placeholder_id(&mut self, spec: Option<(&'static Interface, u32)>) -> ObjectId {
        self.handle.placeholder_id(spec)
    }

    /// Sends a request to the server
    ///
    /// Returns an error if the sender ID of the provided message is no longer valid.
    ///
    /// **Panic:**
    ///
    /// Several checks against the protocol specification are done, and this method will panic if they do
    /// not pass:
    ///
    /// - the message opcode must be valid for the sender interface
    /// - the argument list must match the prototype for the message associated with this opcode
    /// - if the method creates a new object, a [`placeholder_id()`](Handle::placeholder_id) must be given
    ///   in the argument list, either without a specification, or with a specification that matches the
    ///   interface and version deduced from the protocol rules
    pub fn send_request(
        &mut self,
        msg: Message<ObjectId>,
        data: Option<Arc<dyn ObjectData>>,
    ) -> Result<ObjectId, InvalidId> {
        self.handle.send_request(msg, data)
    }

    /// Access the object data associated with a given object ID
    ///
    /// Returns an error if the object ID is not longer valid or if it corresponds to a Wayland
    /// object that is not managed by this backend (when multiple libraries share the same Wayland
    /// socket via `libwayland` if using the system backend).
    pub fn get_data(&self, id: ObjectId) -> Result<Arc<dyn ObjectData>, InvalidId> {
        self.handle.get_data(id)
    }

    /// Set the object data associated with a given object ID
    ///
    /// Returns an error if the object ID is not longer valid or if it corresponds to a Wayland
    /// object that is not managed by this backend (when multiple libraries share the same Wayland
    /// socket via `libwayland` if using the system backend).
    pub fn set_data(&mut self, id: ObjectId, data: Arc<dyn ObjectData>) -> Result<(), InvalidId> {
        self.handle.set_data(id, data)
    }
}

pub(crate) struct DumbObjectData;

impl ObjectData for DumbObjectData {
    #[cfg_attr(coverage, no_coverage)]
    fn event(
        self: Arc<Self>,
        _handle: &mut Handle,
        _msg: Message<ObjectId>,
    ) -> Option<Arc<dyn ObjectData>> {
        unreachable!()
    }

    #[cfg_attr(coverage, no_coverage)]
    fn destroyed(&self, _object_id: ObjectId) {
        unreachable!()
    }
}

pub(crate) struct UninitObjectData;

impl ObjectData for UninitObjectData {
    #[cfg_attr(coverage, no_coverage)]
    fn event(
        self: Arc<Self>,
        _handle: &mut Handle,
        msg: Message<ObjectId>,
    ) -> Option<Arc<dyn ObjectData>> {
        panic!("Received a message on an uninitialized object: {:?}", msg);
    }

    #[cfg_attr(coverage, no_coverage)]
    fn destroyed(&self, _object_id: ObjectId) {}

    #[cfg_attr(coverage, no_coverage)]
    fn debug(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UninitObjectData").finish()
    }
}
