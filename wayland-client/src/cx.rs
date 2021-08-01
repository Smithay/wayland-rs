use std::{
    env,
    io::ErrorKind,
    os::unix::net::UnixStream,
    os::unix::prelude::FromRawFd,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, MutexGuard,
    },
};

use wayland_backend::{
    client::{Backend, Handle, InvalidId, ObjectData, ObjectId, WaylandError},
    protocol::{Interface, ObjectInfo},
};

use nix::{fcntl, Error};

use crate::{oneshot_sink, proxy_internals::ProxyData, Proxy};

#[derive(Clone)]
pub struct Connection {
    backend: Arc<Mutex<Backend>>,
}

impl Connection {
    pub fn handle(&self) -> ConnectionHandle {
        ConnectionHandle { inner: HandleInner::Guard(self.backend.lock().unwrap()) }
    }

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
        Ok(Connection { backend: Arc::new(Mutex::new(backend)) })
    }

    pub fn from_backend(backend: Arc<Mutex<Backend>>) -> Connection {
        Connection { backend }
    }

    pub fn backend(&self) -> Arc<Mutex<Backend>> {
        self.backend.clone()
    }

    pub fn flush(&self) -> Result<(), WaylandError> {
        self.backend.lock().unwrap().flush()
    }

    pub fn dispatch_events(&self) -> Result<usize, WaylandError> {
        self.backend.lock().unwrap().dispatch_events()
    }

    pub fn blocking_dispatch(&self) -> Result<usize, WaylandError> {
        blocking_dispatch_impl(&mut self.backend.lock().unwrap())
    }

    pub fn roundtrip(&self) -> Result<usize, WaylandError> {
        let mut backend = self.backend.lock().unwrap();
        let done = Arc::new(AtomicBool::new(false));
        {
            let mut handle = ConnectionHandle::from_handle(backend.handle());
            let display = handle.display();
            let cb_done = done.clone();
            let sync_data =
                oneshot_sink!(crate::protocol::wl_callback::WlCallback, move |_, _, _| {
                    cb_done.store(true, Ordering::Release);
                });
            display
                .sync(&mut handle, Some(sync_data))
                .map_err(|_| WaylandError::Io(Error::EPIPE.into()))?;
        }

        let mut dispatched = 0;

        while !done.load(Ordering::Acquire) {
            dispatched += blocking_dispatch_impl(&mut backend)?;
        }

        Ok(dispatched)
    }
}

fn blocking_dispatch_impl(backend: &mut Backend) -> Result<usize, WaylandError> {
    backend.flush()?;

    // first, try to dispatch
    match backend.dispatch_events() {
        Ok(n) => return Ok(n),
        Err(WaylandError::Io(e)) if e.kind() == ErrorKind::WouldBlock => {}
        Err(e) => return Err(e),
    }

    // there is nothing to dispatch, wait for readiness
    loop {
        let mut fds = [nix::poll::PollFd::new(
            backend.connection_fd(),
            nix::poll::PollFlags::POLLIN | nix::poll::PollFlags::POLLERR,
        )];
        match nix::poll::poll(&mut fds, -1) {
            Ok(_) => break,
            Err(nix::errno::Errno::EINTR) => continue,
            Err(e) => return Err(WaylandError::Io(e.into())),
        }
    }

    // at this point the fd is ready
    match backend.dispatch_events() {
        Ok(n) => Ok(n),
        // if we are still "wouldblock", that means that there was a dispatch from an other
        // thread with the C-based backend, spuriously return 0.
        Err(WaylandError::Io(e)) if e.kind() == ErrorKind::WouldBlock => Ok(0),
        Err(e) => Err(e),
    }
}

pub struct ConnectionHandle<'a> {
    inner: HandleInner<'a>,
}

enum HandleInner<'a> {
    Handle(&'a mut Handle),
    Guard(MutexGuard<'a, Backend>),
}

impl<'a> HandleInner<'a> {
    #[inline]
    fn handle(&mut self) -> &mut Handle {
        match self {
            HandleInner::Handle(handle) => handle,
            HandleInner::Guard(guard) => guard.handle(),
        }
    }
}

impl<'a> ConnectionHandle<'a> {
    pub(crate) fn from_handle(handle: &'a mut Handle) -> ConnectionHandle<'a> {
        ConnectionHandle { inner: HandleInner::Handle(handle) }
    }

    pub fn send_request<I: Proxy>(
        &mut self,
        proxy: &I,
        request: I::Request,
        data: Option<Arc<ProxyData>>,
    ) -> Result<ObjectId, InvalidId> {
        let msg = proxy.write_request(self, request)?;
        self.inner.handle().send_request(msg, data.map(|arc| arc as Arc<dyn ObjectData>))
    }

    pub fn display(&mut self) -> crate::protocol::wl_display::WlDisplay {
        let display_id = self.inner.handle().display_id();
        Proxy::from_id(self, display_id).unwrap()
    }

    pub fn placeholder_id(&mut self, spec: Option<(&'static Interface, u32)>) -> ObjectId {
        self.inner.handle().placeholder_id(spec)
    }

    pub fn null_id(&mut self) -> ObjectId {
        self.inner.handle().null_id()
    }

    pub fn get_proxy_data(&mut self, id: ObjectId) -> Result<Arc<ProxyData>, InvalidId> {
        self.inner.handle().get_data(id)?.downcast_arc().map_err(|_| InvalidId)
    }

    pub fn object_info(&mut self, id: ObjectId) -> Result<ObjectInfo, InvalidId> {
        self.inner.handle().info(id)
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ConnectError {
    #[error("The wayland library could not be loaded")]
    NoWaylandLib,
    #[error("Could not find wayland compositor")]
    NoCompositor,
    #[error("WAYLAND_SOCKET was set but contained garbage")]
    InvalidFd,
}
