use std::{
    fmt,
    os::unix::{io::RawFd, net::UnixStream},
    sync::Arc,
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
        backend: &Backend,
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
#[derive(Clone, PartialEq, Eq, Hash)]
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

/// A Wayland client backend
///
/// This type hosts all the interface for interacting with the wayland protocol. It can be
/// cloned, all clones refer to the same underlying connection.
#[derive(Clone, Debug)]
pub struct Backend {
    pub(crate) backend: client_impl::InnerBackend,
}

/// A weak handle to a [`Backend`]
///
/// This handle behaves similarly to [`Weak`](std::sync::Weak), and can be used to keep access to
/// the backend without actually preventing it from being dropped.
#[derive(Clone, Debug)]
pub struct WeakBackend {
    inner: client_impl::WeakInnerBackend,
}

impl WeakBackend {
    /// Try to upgrade this weak handle to a [`Backend`]
    ///
    /// Returns `None` if the associated backend was already dropped.
    pub fn upgrade(&self) -> Option<Backend> {
        self.inner.upgrade().map(|backend| Backend { backend })
    }
}

impl Backend {
    /// Try to initialize a Wayland backend on the provided unix stream
    ///
    /// The provided stream should correspond to an already established unix connection with
    /// the Wayland server. On this rust backend, this method never fails.
    pub fn connect(stream: UnixStream) -> Result<Self, NoWaylandLib> {
        client_impl::InnerBackend::connect(stream).map(|backend| Self { backend })
    }

    /// Get a [`WeakBackend`] from this backend
    pub fn downgrade(&self) -> WeakBackend {
        WeakBackend { inner: self.backend.downgrade() }
    }

    /// Flush all pending outgoing requests to the server
    pub fn flush(&self) -> Result<(), WaylandError> {
        self.backend.flush()
    }

    /// Get the object ID for the `wl_display`
    #[inline]
    pub fn display_id(&self) -> ObjectId {
        self.backend.display_id()
    }

    /// Get the last error that occurred on this backend
    ///
    /// If this returns an error, your Wayland connection is already dead.
    #[inline]
    pub fn last_error(&self) -> Option<WaylandError> {
        self.backend.last_error()
    }

    /// Get the detailed information about a wayland object
    ///
    /// Returns an error if the provided object ID is no longer valid.
    #[inline]
    pub fn info(&self, id: ObjectId) -> Result<ObjectInfo, InvalidId> {
        self.backend.info(id)
    }

    /// Create a null object ID
    ///
    /// This object ID is always invalid, and can be used as placeholder.
    #[inline]
    pub fn null_id() -> ObjectId {
        client_impl::InnerBackend::null_id()
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
    /// - if the method creates a new object, a [`null_id()`](Backend::null_id) must be given
    ///   in the argument list at the appropriate place, and a `child_spec` (interface and version)
    ///   can be provided. If one is provided, it'll be checked agains the protocol spec.
    pub fn send_request(
        &self,
        msg: Message<ObjectId>,
        data: Option<Arc<dyn ObjectData>>,
        child_spec: Option<(&'static Interface, u32)>,
    ) -> Result<ObjectId, InvalidId> {
        self.backend.send_request(msg, data, child_spec)
    }

    /// Access the object data associated with a given object ID
    ///
    /// Returns an error if the object ID is not longer valid or if it corresponds to a Wayland
    /// object that is not managed by this backend (when multiple libraries share the same Wayland
    /// socket via `libwayland` if using the system backend).
    pub fn get_data(&self, id: ObjectId) -> Result<Arc<dyn ObjectData>, InvalidId> {
        self.backend.get_data(id)
    }

    /// Set the object data associated with a given object ID
    ///
    /// Returns an error if the object ID is not longer valid or if it corresponds to a Wayland
    /// object that is not managed by this backend (when multiple libraries share the same Wayland
    /// socket via `libwayland` if using the system backend).
    pub fn set_data(&self, id: ObjectId, data: Arc<dyn ObjectData>) -> Result<(), InvalidId> {
        self.backend.set_data(id, data)
    }

    /// Create a new reading guard
    ///
    /// This call will not block, but event callbacks may be invoked in the process
    /// of preparing the guard.
    #[inline]
    pub fn prepare_read(&self) -> Result<ReadEventsGuard, WaylandError> {
        client_impl::InnerReadEventsGuard::try_new(self.backend.clone())
            .map(|guard| ReadEventsGuard { guard })
    }
}

/// Guard for synchronizing event reading across multiple threads
///
/// If multiple threads need to read events from the Wayland socket concurrently,
/// it is necessary to synchronize their access. Failing to do so may cause some of the
/// threads to not be notified of new events, and sleep much longer than appropriate.
///
/// To correctly synchronize access, this type should be used. The guard is created using
/// the [`Backend::prepare_read()`](Backend::prepare_read) method. And the event reading is
/// triggered by consuming the guard using the [`read()`](ReadEventsGuard::read) method.
///
/// If you plan to poll the Wayland socket for readiness, the file descriptor can be retrieved via
/// the [`connection_fd`](ReadEventsGuard::connection_fd) method. Note that for the synchronization to
/// correctly occur, you must *always* create the `ReadEventsGuard` *before* polling the socket.
#[derive(Debug)]
pub struct ReadEventsGuard {
    pub(crate) guard: client_impl::InnerReadEventsGuard,
}

impl ReadEventsGuard {
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
pub(crate) struct DumbObjectData;

impl ObjectData for DumbObjectData {
    #[cfg_attr(coverage, no_coverage)]
    fn event(
        self: Arc<Self>,
        _handle: &Backend,
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
        _handle: &Backend,
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
