use std::{
    env,
    ffi::{OsStr, OsString},
    fs::File,
    io,
    ops::Range,
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

#[derive(Debug)]
pub struct ListeningSocket {
    listener: UnixListener,
    _lock: File,
    socket_path: PathBuf,
    lock_path: PathBuf,
    socket_name: OsString,
}

impl ListeningSocket {
    pub fn bind<S: AsRef<OsStr>>(socket_name: S) -> Result<ListeningSocket, BindError> {
        let runtime_dir: PathBuf =
            env::var("XDG_RUNTIME_DIR").map_err(|_| BindError::RuntimeDirNotSet)?.into();
        let socket_path = runtime_dir.join(socket_name.as_ref());
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

        Ok(ListeningSocket {
            listener,
            _lock,
            socket_path,
            lock_path,
            socket_name: socket_name.as_ref().to_owned(),
        })
    }

    pub fn bind_auto(basename: &str, range: Range<usize>) -> Result<Self, BindError> {
        for i in range {
            // early return on any error except AlreadyInUse
            match ListeningSocket::bind(&format!("{}-{}", basename, i)) {
                Ok(socket) => return Ok(socket),
                Err(BindError::RuntimeDirNotSet) => return Err(BindError::RuntimeDirNotSet),
                Err(BindError::PermissionDenied) => return Err(BindError::PermissionDenied),
                Err(BindError::Io(e)) => return Err(BindError::Io(e)),
                Err(BindError::AlreadyInUse) => {}
            }
        }
        Err(BindError::AlreadyInUse)
    }

    #[must_use = "the client must be initialized by the display using `Display::insert_client` or else the client will hang forever"]
    pub fn accept(&self) -> io::Result<Option<UnixStream>> {
        match self.listener.accept() {
            Ok((stream, _)) => Ok(Some(stream)),
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Returns the name of the listening socket.
    pub fn socket_name(&self) -> &OsStr {
        &self.socket_name
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

#[derive(Debug, thiserror::Error)]
pub enum BindError {
    #[error("Environment variable XDG_RUNTIME_DIR is not set")]
    RuntimeDirNotSet,
    #[error("Could not write to XDG_RUNTIME_DIR")]
    PermissionDenied,
    #[error("Requested socket name is already in use")]
    AlreadyInUse,
    #[error("I/O error: {0}")]
    Io(#[source] io::Error),
}
