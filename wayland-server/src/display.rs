use std::cell::RefCell;
use std::env;
use std::ffi::{OsStr, OsString};
use std::io::{Error as IoError, ErrorKind, Result as IoResult};
use std::os::unix::io::{IntoRawFd, RawFd};
use std::path::PathBuf;
use std::rc::{Rc, Weak};

#[cfg(feature = "native_lib")]
use wayland_sys::server::wl_display;

use imp::DisplayInner;

use {Client, Global, Interface, NewResource};

use calloop::LoopHandle;

/// The wayland display
///
/// This is the core of your wayland server, this object must
/// be kept alive as long as your server is running. It allows
/// you to manage listening sockets and clients.
pub struct Display {
    inner: Rc<RefCell<DisplayInner>>,
}

/// A token that is required for providing non-Send implementations to resources
///
/// This is used to ensure you are indeed on the right thread.
///
/// See `NewResource::implement_nonsend()`.
#[derive(Clone)]
pub struct DisplayToken {
    inner: Weak<RefCell<DisplayInner>>,
}

impl DisplayToken {
    pub(crate) fn upgrade(&self) -> Option<Rc<RefCell<DisplayInner>>> {
        Weak::upgrade(&self.inner)
    }
}

impl Display {
    /// Create a new display
    ///
    /// This method provides you a `Display` and inserts it into an existing
    /// `calloop::EventLoop`.
    ///
    /// Note that at this point, your server is not yet ready to receive connections,
    /// your need to add listening sockets using the `add_socket*` methods.
    pub fn new<Data: 'static>(handle: LoopHandle<Data>) -> Display {
        Display {
            inner: DisplayInner::new(handle),
        }
    }

    /// Get a `DisplayToken` for make non-send implementations
    ///
    /// This is required by `NewResource::implement_nonsend`.
    pub fn get_token(&self) -> DisplayToken {
        DisplayToken {
            inner: Rc::downgrade(&self.inner),
        }
    }

    /// Create a new global object
    ///
    /// This object will be advertised to all clients, and they will
    /// be able to instantiate it from their registries.
    ///
    /// Your implementation will be called whenever a client instantiates
    /// this global.
    ///
    /// The version specified is the **highest supported version**, you must
    /// be able to handle clients that choose to instantiate this global with
    /// a lower version number.
    pub fn create_global<I: Interface, F>(&mut self, version: u32, implementation: F) -> Global<I>
    where
        F: FnMut(NewResource<I>, u32) + 'static,
    {
        assert!(
            version <= I::VERSION,
            "Cannot create global {} with version {}, maximum protocol version is {}.",
            I::NAME,
            version,
            I::VERSION
        );
        Global::create(
            self.inner
                .borrow_mut()
                .create_global(version, implementation, None::<fn(_) -> bool>),
        )
    }

    /// Create a new global object with a filter
    ///
    /// This object will be advertised to all clients, and they will
    /// be able to instantiate it from their registries.
    ///
    /// Your implementation will be called whenever a client instantiates
    /// this global.
    ///
    /// The version specified is the **highest supported version**, you must
    /// be able to handle clients that choose to instantiate this global with
    /// a lower version number.
    pub fn create_global_with_filter<I: Interface, F1, F2>(
        &mut self,
        version: u32,
        implementation: F1,
        mut filter: F2,
    ) -> Global<I>
    where
        F1: FnMut(NewResource<I>, u32) + 'static,
        F2: FnMut(Client) -> bool + 'static,
    {
        assert!(
            version <= I::VERSION,
            "Cannot create global {} with version {}, maximum protocol version is {}.",
            I::NAME,
            version,
            I::VERSION
        );
        Global::create(self.inner.borrow_mut().create_global(
            version,
            implementation,
            Some(move |client_inner| filter(Client::make(client_inner))),
        ))
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
