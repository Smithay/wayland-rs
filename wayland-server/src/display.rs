use std::cell::RefCell;
use std::env;
use std::ffi::{OsStr, OsString};
use std::io::{Error as IoError, ErrorKind, Result as IoResult};
use std::os::unix::io::{IntoRawFd, RawFd};
use std::path::PathBuf;
use std::rc::Rc;

#[cfg(feature = "native_lib")]
use wayland_sys::server::wl_display;

use imp::DisplayInner;

use {Client, EventLoop, Global, Implementation, Interface, LoopToken, NewResource};

/// The wayland display
///
/// This is the core of your wayland server, this object must
/// be kept alive as long as your server is running. It allows
/// you to manage listening sockets and clients.
pub struct Display {
    inner: Rc<RefCell<DisplayInner>>,
}

impl Display {
    /// Create a new display
    ///
    /// This method provides you a `Display` as well as the main `EventLoop`
    /// which will host your clients' objects.
    ///
    /// Note that at this point, your server is not yet ready to receive connections,
    /// your need to add listening sockets using the `add_socket*` methods.
    pub fn new() -> (Display, EventLoop) {
        let (display_inner, evl_inner) = DisplayInner::new();

        (Display { inner: display_inner }, EventLoop::make(evl_inner))
    }

    /// Create a new global object
    ///
    /// This object will be advertized to all clients, and they will
    /// be able to instanciate it from their registries.
    ///
    /// Your implementation will be called whenever a client instanciates
    /// this global.
    ///
    /// The version specified is the **highest supported version**, you must
    /// be able to handle clients that choose to instanciate this global with
    /// a lower version number.
    pub fn create_global<I: Interface, Impl>(
        &mut self,
        token: &LoopToken,
        version: u32,
        implementation: Impl,
    ) -> Global<I>
    where
        Impl: Implementation<NewResource<I>, u32> + 'static,
    {
        Global::create(
            self.inner
                .borrow_mut()
                .create_global(&*token.inner, version, implementation),
        )
    }

    /// Flush events to the clients
    ///
    /// Will send as many pending events as possible to the respective sockets of the clients.
    /// Will not block, but might not send everything if the socket buffer fills up.
    pub fn flush_clients(&self) {
        self.inner.borrow_mut().flush_clients()
    }
}

impl Display {
    /// Add a listening socket to this display
    ///
    /// Wayland clients will be able to connect to your compositor from this socket.
    ///
    /// Socket will be created in the directory specified by the environment variable
    /// `XDG_RUNTIME_DIR`.
    ///
    /// If a name is provided, it is used. Otherwise, if `WAYLAND_DISPLAY` environment
    /// variable is set, its contents are used as socket name. Otherwise, `wayland-0` is used.
    ///
    /// Errors if `name` contains an interior null, or if `XDG_RUNTIME_DIR` is not set,
    /// or if specified could not be bound (either it is already used or the compositor
    /// does not have the rights to create it).
    pub fn add_socket<S>(&mut self, name: Option<S>) -> IoResult<()>
    where
        S: AsRef<OsStr>,
    {
        self.inner.borrow_mut().add_socket(name)
    }

    /// Add an automatically named listening socket to this display
    ///
    /// Wayland clients will be able to connect to your compositor from this socket.
    ///
    /// Socket will be created in the directory specified by the environment variable
    /// `XDG_RUNTIME_DIR`. The directory is scanned for any name in the form `wayland-$d` with
    /// `0 <= $d < 32` and the first available one is used.
    ///
    /// Errors if `XDG_RUNTIME_DIR` is not set, or all 32 names are already in use.
    pub fn add_socket_auto(&mut self) -> IoResult<OsString> {
        self.inner.borrow_mut().add_socket_auto()
    }

    /// Add existing listening socket to this display
    ///
    /// Wayland clients will be able to connect to your compositor from this socket.
    ///
    /// The existing socket fd must already be created, opened, and locked.
    /// The fd must be properly set to CLOEXEC and bound to a socket file
    /// with both bind() and listen() already called. An error is returned
    /// otherwise.
    pub fn add_socket_from<T>(&mut self, socket: T) -> IoResult<()>
    where
        T: IntoRawFd,
    {
        unsafe { self.add_socket_fd(socket.into_raw_fd()) }
    }

    /// Add existing listening socket to this display from a raw file descriptor
    ///
    /// Wayland clients will be able to connect to your compositor from this socket.
    ///
    /// The library takes ownership of the provided socket if this method returns
    /// successfully.
    ///
    /// The existing socket fd must already be created, opened, and locked.
    /// The fd must be properly set to CLOEXEC and bound to a socket file
    /// with both bind() and listen() already called. An error is returned
    /// otherwise.
    pub unsafe fn add_socket_fd(&self, fd: RawFd) -> IoResult<()> {
        self.inner.borrow_mut().add_socket_fd(fd)
    }

    /// Create a new client to this display from an already-existing connected Fd
    pub unsafe fn create_client(&self, fd: RawFd) -> Client {
        Client::make(self.inner.borrow_mut().create_client(fd))
    }
}

#[cfg(feature = "native_lib")]
impl Display {
    /// Retrieve a pointer from the C lib to this `wl_display`
    pub fn c_ptr(&self) -> *mut wl_display {
        self.inner.borrow().ptr()
    }
}

pub(crate) fn get_runtime_dir() -> IoResult<PathBuf> {
    match env::var_os("XDG_RUNTIME_DIR") {
        Some(s) => Ok(s.into()),
        None => Err(IoError::new(
            ErrorKind::NotFound,
            "XDG_RUNTIME_DIR env variable is not set",
        )),
    }
}
