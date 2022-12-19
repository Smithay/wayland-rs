use std::{
    env,
    ffi::{OsStr, OsString},
    fs::File,
    io,
    os::unix::{
        io::{AsRawFd, FromRawFd, RawFd},
        net::{UnixListener, UnixStream},
    },
    path::PathBuf,
};

use nix::{
    fcntl::{flock, open, FlockArg, OFlag},
    sys::stat::{lstat, Mode},
    unistd::unlink,
};

/// An utility representing a unix socket on which your compositor is listening for new clients
#[derive(Debug)]
pub struct ListeningSocket {
    listener: UnixListener,
    _lock: File,
    socket_path: PathBuf,
    lock_path: PathBuf,
    socket_name: Option<OsString>,
}

impl ListeningSocket {
    /// Attempt to bind a listening socket with given name
    ///
    /// This method will acquire an associate lockfile. The socket will be created in the
    /// directory pointed to by the `XDG_RUNTIME_DIR` environment variable.
    pub fn bind<S: AsRef<OsStr>>(socket_name: S) -> Result<Self, BindError> {
        let runtime_dir: PathBuf =
            env::var("XDG_RUNTIME_DIR").map_err(|_| BindError::RuntimeDirNotSet)?.into();
        if !runtime_dir.is_absolute() {
            return Err(BindError::RuntimeDirNotSet);
        }
        let socket_path = runtime_dir.join(socket_name.as_ref());
        let mut socket = Self::bind_absolute(socket_path)?;
        socket.socket_name = Some(socket_name.as_ref().into());
        Ok(socket)
    }

    /// Attempt to bind a listening socket from a sequence of names
    ///
    /// This method will repeatedly try to bind sockets in teh form `{basename}-{n}` for values of `n`
    /// yielded from the provided range and returns the first one that succeeds.
    ///
    /// This method will acquire an associate lockfile. The socket will be created in the
    /// directory pointed to by the `XDG_RUNTIME_DIR` environment variable.
    pub fn bind_auto(
        basename: &str,
        range: impl IntoIterator<Item = usize>,
    ) -> Result<Self, BindError> {
        for i in range {
            // early return on any error except AlreadyInUse
            match Self::bind(format!("{}-{}", basename, i)) {
                Ok(socket) => return Ok(socket),
                Err(BindError::RuntimeDirNotSet) => return Err(BindError::RuntimeDirNotSet),
                Err(BindError::PermissionDenied) => return Err(BindError::PermissionDenied),
                Err(BindError::Io(e)) => return Err(BindError::Io(e)),
                Err(BindError::AlreadyInUse) => {}
            }
        }
        Err(BindError::AlreadyInUse)
    }

    /// Attempt to bind a listening socket with given name
    ///
    /// The socket will be created at the specified path, and this method will acquire an associatet lockfile
    /// alongside it.
    pub fn bind_absolute(socket_path: PathBuf) -> Result<Self, BindError> {
        let lock_path = socket_path.with_extension("lock");

        // open the lockfile
        let lock_fd = open(
            &lock_path,
            OFlag::O_CREAT | OFlag::O_CLOEXEC | OFlag::O_RDWR,
            Mode::S_IRUSR | Mode::S_IWUSR | Mode::S_IRGRP | Mode::S_IWGRP,
        )
        .map_err(|_| BindError::PermissionDenied)?;

        // SAFETY: We have just opened the file descriptor.
        let _lock = unsafe { File::from_raw_fd(lock_fd) };

        // lock the lockfile
        if flock(lock_fd, FlockArg::LockExclusiveNonblock).is_err() {
            return Err(BindError::AlreadyInUse);
        }

        // check if an old socket exists, and cleanup if relevant
        match lstat(&socket_path) {
            Err(nix::Error::ENOENT) => {
                // none exist, good
            }
            Ok(_) => {
                // one exist, remove it
                unlink(&socket_path).map_err(|_| BindError::AlreadyInUse)?;
            }
            Err(e) => {
                // some error stat-ing the socket?
                return Err(BindError::Io(e.into()));
            }
        }

        // At this point everything is good to start listening on the socket
        let listener = UnixListener::bind(&socket_path).map_err(BindError::Io)?;

        listener.set_nonblocking(true).map_err(BindError::Io)?;

        Ok(Self { listener, _lock, socket_path, lock_path, socket_name: None })
    }

    /// Try to accept a new connection to the listening socket
    ///
    /// This method will never block, and return `Ok(None)` if no new connection is available.
    #[must_use = "the client must be initialized by the display using `Display::insert_client` or else the client will hang forever"]
    pub fn accept(&self) -> io::Result<Option<UnixStream>> {
        match self.listener.accept() {
            Ok((stream, _)) => Ok(Some(stream)),
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Returns the name of the listening socket.
    ///
    /// Will only be [`Some`] if that socket was created with [`bind`](ListeningSocket::bind) or
    /// [`bind_auto`](ListeningSocket::bind_auto).
    pub fn socket_name(&self) -> Option<&OsStr> {
        self.socket_name.as_deref()
    }
}

impl AsRawFd for ListeningSocket {
    /// Returns a file descriptor that may be polled for readiness.
    ///
    /// This file descriptor may be polled using apis such as epoll and kqueue to be told when a client has
    /// found the socket and is trying to connect.
    ///
    /// When the polling system reports the file descriptor is ready, you can use [`ListeningSocket::accept`]
    /// to get a stream to the new client.
    fn as_raw_fd(&self) -> RawFd {
        self.listener.as_raw_fd()
    }
}

impl Drop for ListeningSocket {
    fn drop(&mut self) {
        let _ = unlink(&self.socket_path);
        let _ = unlink(&self.lock_path);
    }
}

/// Error that can occur when trying to bind a [`ListeningSocket`]
#[derive(Debug, thiserror::Error)]
pub enum BindError {
    /// The Environment variable `XDG_RUNTIME_DIR` is not set
    #[error("Environment variable XDG_RUNTIME_DIR is not set or invalid")]
    RuntimeDirNotSet,
    /// The application was not able to create a file in `XDG_RUNTIME_DIR`
    #[error("Could not write to XDG_RUNTIME_DIR")]
    PermissionDenied,
    /// The requested socket name is already in use
    #[error("Requested socket name is already in use")]
    AlreadyInUse,
    /// Some other IO error occured
    #[error("I/O error: {0}")]
    Io(#[source] io::Error),
}
