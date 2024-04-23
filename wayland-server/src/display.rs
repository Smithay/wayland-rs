use std::{
    os::unix::io::{AsFd, AsRawFd, BorrowedFd},
    os::unix::net::UnixStream,
    sync::Arc,
};

use wayland_backend::{
    protocol::ObjectInfo,
    server::{Backend, ClientData, GlobalId, Handle, InitError, InvalidId, ObjectId},
};

use crate::{
    global::{GlobalData, GlobalDispatch},
    Client, Resource,
};

/// The Wayland display
///
/// This struct is the core of your Wayland compositor. You'll use it in your event loop to drive the
/// protocol processing of all your clients. All other interactions with the protocol itself are done
/// through the [`DisplayHandle`] struct, on which the `State` type parameter is erased for convenience.
///
/// ## Usage
///
/// The main loop of a Wayland compositor generally needs to wait on several sources of events, using
/// tools like `epoll` (on Linux). The Wayland display can be integrated in this mechanism by getting the
/// file descriptor as from [`.backend()`][Self::backend()][`.poll_fd()`][Backend::poll_fd()] and invoking
/// the [`dispatch_clients()`][Self::dispatch_clients()] method whenever it becomes readable.
///
/// To ensure all clients receive the events your compositor sends them, you also need to regularly invoke
/// the [`flush_clients()`][Self::flush_clients()] method, which will write the outgoing buffers into the
/// sockets.
#[derive(Debug)]
pub struct Display<State: 'static> {
    backend: Backend<State>,
}

impl<State: 'static> Display<State> {
    /// Create a new Wayland display
    ///
    /// Can only fail if both the `server_system` and `dlopen` features of `wayland-backend` were enabled,
    /// and the `libwayland-server.so` library could not be found.
    pub fn new() -> Result<Display<State>, InitError> {
        Ok(Display { backend: Backend::new()? })
    }

    /// Retrieve a [`DisplayHandle`] for this [`Display`].
    ///
    /// This is the type with which all of your interactions with the Wayland protocol are done.
    pub fn handle(&self) -> DisplayHandle {
        DisplayHandle { handle: self.backend.handle() }
    }

    /// Dispatch all requests received from clients to their respective callbacks.
    ///
    /// The `state` argument is the main state of your compositor, which will be accessible from most of your
    /// callbacks.
    pub fn dispatch_clients(&mut self, state: &mut State) -> std::io::Result<usize> {
        self.backend.dispatch_all_clients(state)
    }

    /// Flush outgoing buffers into their respective sockets.
    pub fn flush_clients(&mut self) -> std::io::Result<()> {
        self.backend.flush(None)
    }

    /// Access the underlying [`Backend`] of this [`Display`]
    pub fn backend(&mut self) -> &mut Backend<State> {
        &mut self.backend
    }
}

impl<State> AsFd for Display<State> {
    /// Provides fd from [`Backend::poll_fd`] for polling.
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.backend.poll_fd()
    }
}

/// A handle to the Wayland display
///
/// A display handle may be constructed from a [`Handle`] using it's [`From`] implementation.
#[derive(Clone)]
pub struct DisplayHandle {
    pub(crate) handle: Handle,
}

impl std::fmt::Debug for DisplayHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DisplayHandle").finish_non_exhaustive()
    }
}

impl DisplayHandle {
    /// Returns the underlying [`Handle`] from `wayland-backend`.
    pub fn backend_handle(&self) -> Handle {
        self.handle.clone()
    }

    /// Insert a new client in your [`Display`]
    ///
    /// This client will be associated with the provided [`ClientData`], that you can then retrieve from
    /// it via [`Client::get_data()`], and its requests will be processed by the [`Display`] and your
    /// callbacks.
    pub fn insert_client(
        &mut self,
        stream: UnixStream,
        data: Arc<dyn ClientData>,
    ) -> std::io::Result<Client> {
        let id = self.handle.insert_client(stream, data.clone())?;
        Ok(Client { id, data })
    }

    /// Retrieve the [`Client`] which owns the object represented by the given ID
    pub fn get_client(&self, id: ObjectId) -> Result<Client, InvalidId> {
        let client_id = self.handle.get_client(id)?;
        Client::from_id(self, client_id)
    }

    /// Create a new protocol global
    ///
    /// This global will be advertized to clients through the `wl_registry` according to the rules
    /// defined by your [`GlobalDispatch`] implementation for the given interface. Whenever a client
    /// binds this global, the associated [`GlobalDispatch::bind()`] method will be invoked on your
    /// `State`.
    pub fn create_global<State, I: Resource + 'static, U: Send + Sync + 'static>(
        &self,
        version: u32,
        data: U,
    ) -> GlobalId
    where
        State: GlobalDispatch<I, U> + 'static,
    {
        self.handle.create_global::<State>(
            I::interface(),
            version,
            Arc::new(GlobalData { data, _types: std::marker::PhantomData }),
        )
    }

    /// Disable this global
    ///
    /// Clients will be notified of the global removal, and it will not be advertized to new clients. However
    /// the state associated with this global is not freed, so clients which already know about it can still
    /// bind it.
    pub fn disable_global<State: 'static>(&self, id: GlobalId) {
        self.handle.disable_global::<State>(id)
    }

    /// Remove this global
    ///
    /// Clients will be notified of the global removal if it was not already disabled. The state associated
    /// with this global is freed, meaning clients trying to bind it will receive a protocol error.
    ///
    /// When removing a global, it is recommended to first disable it using
    /// [`disable_global()`][Self::disable_global()] to allow some time for clients to register that
    /// the global is getting removed, to avoid a race where a client would be killed because it bound a global
    /// at the same as the server decided to remove it. After the global has been disabled for some time (like
    /// a few seconds) it should be safe to actually remove it.
    pub fn remove_global<State: 'static>(&self, id: GlobalId) {
        self.handle.remove_global::<State>(id)
    }

    /// Access the protocol information for a Wayland object
    ///
    /// Returns an error if the object is no longer valid.
    pub fn object_info(&self, id: ObjectId) -> Result<ObjectInfo, InvalidId> {
        self.handle.object_info(id)
    }

    /// Send an event to given Wayland object
    ///
    /// This is intended to be a low-level method. You can alternatively use the methods on the
    /// type representing your object, or [`Resource::send_event()`], which may be more convenient.
    pub fn send_event<I: Resource>(
        &self,
        resource: &I,
        event: I::Event<'_>,
    ) -> Result<(), InvalidId> {
        let msg = resource.write_event(self, event)?;
        let msg = msg.map_fd(|fd| fd.as_raw_fd());
        self.handle.send_event(msg)
    }

    /// Trigger a protocol error on this object
    ///
    /// This is intended to be a low-level method. See [`Resource::post_error()`], for a more convenient
    /// method.
    pub fn post_error<I: Resource>(&self, resource: &I, code: u32, error: String) {
        self.handle.post_error(resource.id(), code, std::ffi::CString::new(error).unwrap())
    }

    /// Access the object data associated with this object
    ///
    /// This is intended to be a low-level method. See [`Resource::object_data()`], for a more convenient
    /// method.
    pub fn get_object_data(
        &self,
        id: ObjectId,
    ) -> Result<Arc<dyn std::any::Any + Send + Sync + 'static>, InvalidId> {
        self.handle.get_object_data_any(id)
    }

    /// Flush outgoing buffers into their respective sockets.
    pub fn flush_clients(&mut self) -> std::io::Result<()> {
        self.handle.flush(None)
    }
}

impl From<Handle> for DisplayHandle {
    /// Creates a [`DisplayHandle`] using a [`Handle`] from `wayland-backend`.
    fn from(handle: Handle) -> Self {
        Self { handle }
    }
}
