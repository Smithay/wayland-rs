use std::{
    env,
    io::ErrorKind,
    os::unix::net::UnixStream,
    os::unix::prelude::FromRawFd,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use wayland_backend::{
    client::{Backend, InvalidId, ObjectData, ObjectId, ReadEventsGuard, WaylandError},
    protocol::{ObjectInfo, ProtocolError},
};

use nix::{fcntl, Error};

use crate::{EventQueue, Proxy};

/// The Wayland connection
///
/// This is the main type representing your connection to the Wayland server.
#[derive(Debug, Clone)]
pub struct Connection {
    backend: Backend,
}

impl Connection {
    /// Try to connect to the Wayland server following the environment
    ///
    /// This is the standard way to initialize a Wayland connection.
    pub fn connect_to_env() -> Result<Connection, ConnectError> {
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
            socket_path.push(env::var_os("WAYLAND_DISPLAY").ok_or(ConnectError::NoCompositor)?);

            UnixStream::connect(socket_path).map_err(|_| ConnectError::NoCompositor)?
        };

        let backend = Backend::connect(stream).map_err(|_| ConnectError::NoWaylandLib)?;
        Ok(Connection { backend })
    }

    /// Initialize a Wayland connection from an already existing Unix stream
    pub fn from_socket(stream: UnixStream) -> Result<Connection, ConnectError> {
        let backend = Backend::connect(stream).map_err(|_| ConnectError::NoWaylandLib)?;
        Ok(Connection { backend })
    }

    /// Wrap an existing [`Backend`] into a Connection
    pub fn from_backend(backend: Backend) -> Connection {
        Connection { backend }
    }

    /// Get the [`Backend`] underlying this Connection
    pub fn backend(&self) -> Backend {
        self.backend.clone()
    }

    /// Flush pending outgoing events to the server
    ///
    /// This needs to be done regularly to ensure the server receives all your requests.
    pub fn flush(&self) -> Result<(), WaylandError> {
        self.backend.flush()
    }

    /// Start a synchronized read from the socket
    ///
    /// This is needed if you plan to wait on readiness of the Wayland socket using an event
    /// loop. See [`ReadEventsGuard`] for details. Once the events are received, you'll then
    /// need to dispatch them from the event queue using
    /// [`EventQueue::dispatch_pending()`](EventQueue::dispatch_pending).
    ///
    /// If you don't need to manage multiple event sources, see
    /// [`blocking_dispatch()`](Connection::blocking_dispatch) for a simpler mechanism.
    pub fn prepare_read(&self) -> Result<ReadEventsGuard, WaylandError> {
        self.backend.prepare_read()
    }

    /// Block until events are received from the server
    ///
    /// This will flush the outgoing socket, and then block until events are received from the
    /// server and read them. You'll then need to invoke
    /// [`EventQueue::dispatch_pending()`](EventQueue::dispatch_pending) to dispatch them on
    /// their respective event queues. Alternatively,
    /// [`EventQueue::blocking_dispatch()`](EventQueue::blocking_dispatch) does both.
    pub fn blocking_dispatch(&self) -> Result<usize, WaylandError> {
        blocking_dispatch_impl(self.backend.clone())
    }

    /// Do a roundtrip to the server
    ///
    /// This method will block until the Wayland server has processed and answered all your
    /// preceding requests. This is notably useful during the initial setup of an app, to wait for
    /// the initial state from the server.
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

    /// Create a new event queue
    pub fn new_event_queue<D>(&self) -> EventQueue<D> {
        EventQueue::new(self.clone())
    }

    /// Retrive the protocol error that occured on the socket (if any)
    pub fn protocol_error(&self) -> Option<ProtocolError> {
        match dbg!(self.backend.last_error())? {
            WaylandError::Protocol(err) => Some(err),
            WaylandError::Io(_) => None,
        }
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

impl Connection {
    /// Get the `WlDisplay` associated with this connection
    pub fn display(&self) -> crate::protocol::wl_display::WlDisplay {
        let display_id = self.backend.display_id();
        Proxy::from_id(self, display_id).unwrap()
    }

    /// Send a request associated with the provided object
    ///
    /// This is a low-level interface for sending requests, you will likely instead use
    /// the methods of the types representing each interface.
    pub fn send_request<I: Proxy>(
        &self,
        proxy: &I,
        request: I::Request,
        data: Option<Arc<dyn ObjectData>>,
    ) -> Result<ObjectId, InvalidId> {
        let (msg, child_spec) = proxy.write_request(self, request)?;
        self.backend.send_request(msg, data, child_spec)
    }

    /// Create a null id for request serialization
    ///
    /// This is a low-level interface for sending requests, you don't need to use it if you
    /// are using the methods of the types representing each interface.
    pub fn null_id() -> ObjectId {
        Backend::null_id()
    }

    /// Get the object data for a given object ID
    ///
    /// This is a low-level interface, a higher-level interface for manipulating object data
    /// is given as [`Proxy::data()`](crate::Proxy::data).
    pub fn get_object_data(&self, id: ObjectId) -> Result<Arc<dyn ObjectData>, InvalidId> {
        self.backend.get_data(id)
    }

    /// Get the protocol information related to given object ID
    pub fn object_info(&self, id: ObjectId) -> Result<ObjectInfo, InvalidId> {
        self.backend.info(id)
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
