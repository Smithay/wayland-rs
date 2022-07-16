use std::{
    env,
    io::ErrorKind,
    os::unix::net::UnixStream,
    os::unix::prelude::FromRawFd,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    }, ffi::OsString
};

use wayland_backend::{
    client::{Backend, InvalidId, ObjectData, ObjectId, ReadEventsGuard, WaylandError},
    protocol::{ObjectInfo, ProtocolError},
};

use nix::{fcntl, Error};

use crate::{protocol::wl_display::WlDisplay, EventQueue, Proxy};

/// The Wayland connection
///
/// This is the main type representing your connection to the Wayland server, though most of the interaction
/// with the protocol are actually done using other types. The two main an simple app as for the
/// [`Connection`] are:
///
/// - Obtaining the initial [`WlDisplay`] through the [`display()`](Connection::display) method.
/// - Creating new [`EventQueue`]s with the [`new_event_queue()`](Connection::new_event_queue) method.
///
/// It can be created through the [`connect_to_env()`](Connection::connect_to_env) method to follow the
/// configuration from the environment (which is what you'll do most of the time), or using the
/// [`from_socket()`](Connection::from_socket) method if you retrieved your connected Wayland socket through
/// other means.
///
/// In case you need to plug yourself into an external Wayland connection that you don't control, you'll
/// likely get access to it as a [`Backend`], in which case you can create a [`Connection`] from it using
/// the [`from_backend`](Connection::from_backend) method.
#[derive(Debug, Clone)]
pub struct Connection {
    pub(crate) backend: Backend,
}

impl Connection {
    /// Try to connect to the Wayland server following the environment
    ///
    /// This is the standard way to initialize a Wayland connection.
    pub fn connect_to_env() -> Result<Self, ConnectError> {
        let stream = if let Ok(txt) = env::var("WAYLAND_SOCKET") {
            // We should connect to the provided WAYLAND_SOCKET
            let fd = txt.parse::<i32>().map_err(|_| ConnectError::InvalidFd)?;
            // remove the variable so any child processes don't see it
            env::remove_var("WAYLAND_SOCKET");
            // set the CLOEXEC flag on this FD
            let flags = fcntl::fcntl(fd, fcntl::FcntlArg::F_GETFD);
            let result = flags
                .map(|f| fcntl::FdFlag::from_bits(f).unwrap() | fcntl::FdFlag::FD_CLOEXEC)
                .and_then(|f| fcntl::fcntl(fd, fcntl::FcntlArg::F_SETFD(f)));
            match result {
                Ok(_) => {
                    // setting the O_CLOEXEC worked
                    unsafe { FromRawFd::from_raw_fd(fd) }
                }
                Err(_) => {
                    // something went wrong in F_GETFD or F_SETFD
                    let _ = ::nix::unistd::close(fd);
                    return Err(ConnectError::InvalidFd);
                }
            }
        } else {
            let mut socket_path = env::var_os("XDG_RUNTIME_DIR")
                .map(Into::<PathBuf>::into)
                .ok_or(ConnectError::NoCompositor)?;
            socket_path.push(env::var_os("WAYLAND_DISPLAY").unwrap_or(OsString::from("wayland-0")));

            UnixStream::connect(socket_path).map_err(|_| ConnectError::NoCompositor)?
        };

        let backend = Backend::connect(stream).map_err(|_| ConnectError::NoWaylandLib)?;
        Ok(Self { backend })
    }

    /// Initialize a Wayland connection from an already existing Unix stream
    pub fn from_socket(stream: UnixStream) -> Result<Self, ConnectError> {
        let backend = Backend::connect(stream).map_err(|_| ConnectError::NoWaylandLib)?;
        Ok(Self { backend })
    }

    /// Get the `WlDisplay` associated with this connection
    pub fn display(&self) -> WlDisplay {
        let display_id = self.backend.display_id();
        Proxy::from_id(self, display_id).unwrap()
    }

    /// Create a new event queue
    pub fn new_event_queue<State>(&self) -> EventQueue<State> {
        EventQueue::new(self.clone())
    }

    /// Wrap an existing [`Backend`] into a [`Connection`]
    pub fn from_backend(backend: Backend) -> Self {
        Self { backend }
    }

    /// Get the [`Backend`] underlying this [`Connection`]
    pub fn backend(&self) -> Backend {
        self.backend.clone()
    }

    /// Flush pending outgoing events to the server
    ///
    /// This needs to be done regularly to ensure the server receives all your requests, though several
    /// dispatching methods do it implicitly (this is stated in their documentation when they do).
    pub fn flush(&self) -> Result<(), WaylandError> {
        self.backend.flush()
    }

    /// Start a synchronized read from the socket
    ///
    /// This is needed if you plan to wait on readiness of the Wayland socket using an event loop. See
    /// [`ReadEventsGuard`] for details. Once the events are received, you'll then need to dispatch them from
    /// their event queues using [`EventQueue::dispatch_pending()`](EventQueue::dispatch_pending).
    ///
    /// If you don't need to manage multiple event sources, see
    /// [`blocking_dispatch()`](Connection::blocking_read) for a simpler mechanism. [`EventQueue`] has an
    /// identical method for convenience.
    pub fn prepare_read(&self) -> Result<ReadEventsGuard, WaylandError> {
        self.backend.prepare_read()
    }

    /// Block until events are received from the server
    ///
    /// This will flush the outgoing socket, and then block until events are received from the
    /// server and read them. You'll then need to invoke
    /// [`EventQueue::dispatch_pending()`](EventQueue::dispatch_pending) to dispatch them on
    /// their respective event queues. Alternatively,
    /// [`EventQueue::blocking_dispatch()`](EventQueue::blocking_dispatch) does the same thing as this
    /// method but also dispatches the pending messages on the queue it was invoked.
    ///
    /// If you created objects bypassing the event queues with direct [`ObjectData`] callbacks, those
    /// callbacks will be invoked (if those objects received any events) before this method returns.
    pub fn blocking_read(&self) -> Result<usize, WaylandError> {
        blocking_dispatch_impl(self.backend.clone())
    }

    /// Do a roundtrip to the server
    ///
    /// This method will block until the Wayland server has processed and answered all your
    /// preceding requests. This is notably useful during the initial setup of an app, to wait for
    /// the initial state from the server.
    ///
    /// See [`EventQueue::roundtrip()`] for a version that includes the dispatching of the event queue.
    pub fn roundtrip(&self) -> Result<usize, WaylandError> {
        let done = Arc::new(AtomicBool::new(false));
        {
            let display = self.display();
            let cb_done = done.clone();
            let sync_data = Arc::new(SyncData { done: cb_done });
            self.send_request(
                &display,
                crate::protocol::wl_display::Request::Sync {},
                Some(sync_data),
            )
            .map_err(|_| WaylandError::Io(Error::EPIPE.into()))?;
        }

        let mut dispatched = 0;

        while !done.load(Ordering::Acquire) {
            dispatched += blocking_dispatch_impl(self.backend.clone())?;
        }

        Ok(dispatched)
    }

    /// Retrieve the protocol error that occured on the connection if any
    ///
    /// If this method returns `Some`, it means your Wayland connection is already dead.
    pub fn protocol_error(&self) -> Option<ProtocolError> {
        match dbg!(self.backend.last_error())? {
            WaylandError::Protocol(err) => Some(err),
            WaylandError::Io(_) => None,
        }
    }

    /// Send a request associated with the provided object
    ///
    /// This is a low-level interface used by the code generated by `wayland-scanner`, you will likely
    /// instead use the methods of the types representing each interface, or the [`Proxy::send_request`] and
    /// [`Proxy::send_constructor`]
    pub fn send_request<I: Proxy>(
        &self,
        proxy: &I,
        request: I::Request,
        data: Option<Arc<dyn ObjectData>>,
    ) -> Result<ObjectId, InvalidId> {
        let (msg, child_spec) = proxy.write_request(self, request)?;
        self.backend.send_request(msg, data, child_spec)
    }

    /// Get the protocol information related to given object ID
    pub fn object_info(&self, id: ObjectId) -> Result<ObjectInfo, InvalidId> {
        self.backend.info(id)
    }

    /// Get the object data for a given object ID
    ///
    /// This is a low-level interface used by the code generated by `wayland-scanner`, a higher-level
    /// interface for manipulating the user-data assocated to [`Dispatch`](crate::Dispatch) implementations
    /// is given as [`Proxy::data()`]. Also see [`Proxy::object_data()`].
    pub fn get_object_data(&self, id: ObjectId) -> Result<Arc<dyn ObjectData>, InvalidId> {
        self.backend.get_data(id)
    }
}

pub(crate) fn blocking_dispatch_impl(backend: Backend) -> Result<usize, WaylandError> {
    backend.flush()?;

    // first, prepare the read
    let guard = backend.prepare_read()?;

    // there is nothing to dispatch, wait for readiness
    loop {
        let mut fds = [nix::poll::PollFd::new(
            guard.connection_fd(),
            nix::poll::PollFlags::POLLIN | nix::poll::PollFlags::POLLERR,
        )];
        match nix::poll::poll(&mut fds, -1) {
            Ok(_) => break,
            Err(nix::errno::Errno::EINTR) => continue,
            Err(e) => return Err(WaylandError::Io(e.into())),
        }
    }

    // at this point the fd is ready
    match guard.read() {
        Ok(n) => Ok(n),
        // if we are still "wouldblock", that means that there was a dispatch from an other
        // thread with the C-based backend, spuriously return 0.
        Err(WaylandError::Io(e)) if e.kind() == ErrorKind::WouldBlock => Ok(0),
        Err(e) => Err(e),
    }
}

/// An error when trying to establish a Wayland connection.
#[derive(thiserror::Error, Debug)]
pub enum ConnectError {
    /// The wayland library could not be loaded.
    #[error("The wayland library could not be loaded")]
    NoWaylandLib,

    /// Could not find wayland compositor
    #[error("Could not find wayland compositor")]
    NoCompositor,

    /// `WAYLAND_SOCKET` was set but contained garbage
    #[error("WAYLAND_SOCKET was set but contained garbage")]
    InvalidFd,
}

/*
    wl_callback object data for wl_display.sync
*/

pub(crate) struct SyncData {
    pub(crate) done: Arc<AtomicBool>,
}

impl ObjectData for SyncData {
    fn event(
        self: Arc<Self>,
        _handle: &Backend,
        _msg: wayland_backend::protocol::Message<ObjectId>,
    ) -> Option<Arc<dyn ObjectData>> {
        self.done.store(true, Ordering::Release);
        None
    }

    fn destroyed(&self, _: ObjectId) {}
}
