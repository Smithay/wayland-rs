use std::{
    env,
    ffi::{OsStr, OsString},
    fs::{self, File},
    io,
    os::unix::{
        fs::OpenOptionsExt,
        io::{AsFd, AsRawFd, BorrowedFd, RawFd},
        net::{UnixListener, UnixStream},
        prelude::MetadataExt,
    },
    path::PathBuf,
};

use rustix::fs::{flock, FlockOperation};

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
    /// This method will repeatedly try to bind sockets in the form `{basename}-{n}` for values of `n`
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
            match Self::bind(format!("{basename}-{i}")) {
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
        let mut _lock;

        // The locking code uses a loop to avoid an open()-flock() race condition, described in more
        // detail in the comment below. The implementation roughtly follows the one from libbsd:
        //
        // https://gitlab.freedesktop.org/libbsd/libbsd/-/blob/73b25a8f871b3a20f6ff76679358540f95d7dbfd/src/flopen.c#L71
        loop {
            // open the lockfile
            _lock = File::options()
                .create(true)
                .truncate(true)
                .read(true)
                .write(true)
                .mode(0o660)
                .open(&lock_path)
                .map_err(|_| BindError::PermissionDenied)?;

            // lock the lockfile
            if flock(&_lock, FlockOperation::NonBlockingLockExclusive).is_err() {
                return Err(BindError::AlreadyInUse);
            }

            // Verify that the file we locked is the same as the file on disk. An unlucky unlink()
            // from a different thread which happens right between our open() and flock() may
            // result in us successfully locking a now-nonexistent file, with another thread locking
            // the same-named but newly created lock file, then both threads will think they have
            // exclusive access to the same socket. To prevent this, check that we locked the actual
            // currently existing file.
            let fd_meta = _lock.metadata().map_err(BindError::Io)?;
            let on_disk_meta = match fs::metadata(&lock_path) {
                Ok(meta) => meta,
                Err(err) if err.kind() == io::ErrorKind::NotFound => {
                    // This can happen during the aforementioned race condition.
                    continue;
                }
                Err(err) => return Err(BindError::Io(err)),
            };

            if fd_meta.dev() == on_disk_meta.dev() && fd_meta.ino() == on_disk_meta.ino() {
                break;
            }
        }

        // check if an old socket exists, and cleanup if relevant
        match socket_path.try_exists() {
            Ok(false) => {
                // none exist, good
            }
            Ok(true) => {
                // one exist, remove it
                fs::remove_file(&socket_path).map_err(|_| BindError::AlreadyInUse)?;
            }
            Err(e) => {
                // some error stat-ing the socket?
                return Err(BindError::Io(e));
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
    /// Will only be [`Some`] if that socket was created with [`bind()`][Self::bind()] or
    /// [`bind_auto()`][Self::bind_auto()].
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
    /// When the polling system reports the file descriptor is ready, you can use [`accept()`][Self::accept()]
    /// to get a stream to the new client.
    fn as_raw_fd(&self) -> RawFd {
        self.listener.as_raw_fd()
    }
}

impl AsFd for ListeningSocket {
    /// Returns a file descriptor that may be polled for readiness.
    ///
    /// This file descriptor may be polled using apis such as epoll and kqueue to be told when a client has
    /// found the socket and is trying to connect.
    ///
    /// When the polling system reports the file descriptor is ready, you can use [`accept()`][Self::accept()]
    /// to get a stream to the new client.
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.listener.as_fd()
    }
}

impl Drop for ListeningSocket {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.socket_path);
        let _ = fs::remove_file(&self.lock_path);
    }
}

/// Error that can occur when trying to bind a [`ListeningSocket`]
#[derive(Debug)]
pub enum BindError {
    /// The Environment variable `XDG_RUNTIME_DIR` is not set
    RuntimeDirNotSet,
    /// The application was not able to create a file in `XDG_RUNTIME_DIR`
    PermissionDenied,
    /// The requested socket name is already in use
    AlreadyInUse,
    /// Some other IO error occured
    Io(io::Error),
}

impl std::error::Error for BindError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            BindError::RuntimeDirNotSet => None,
            BindError::PermissionDenied => None,
            BindError::AlreadyInUse => None,
            BindError::Io(source) => Some(source),
        }
    }
}

impl std::fmt::Display for BindError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            BindError::RuntimeDirNotSet => {
                write!(f, "Environment variable XDG_RUNTIME_DIR is not set or invalid")
            }
            BindError::PermissionDenied => write!(f, "Could not write to XDG_RUNTIME_DIR"),
            BindError::AlreadyInUse => write!(f, "Requested socket name is already in use"),
            BindError::Io(source) => write!(f, "I/O error: {source}"),
        }
    }
}
