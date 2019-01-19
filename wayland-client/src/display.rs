use std::env;
use std::ffi::OsString;
use std::io;
use std::ops::Deref;
use std::os::unix::io::{IntoRawFd, RawFd};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::Arc;

use nix::fcntl;

use {EventQueue, Proxy};

use imp::DisplayInner;

#[cfg(feature = "native_lib")]
use wayland_sys::client::wl_display;

/// Enum representing the possible reasons why connecting to the wayland server failed
#[derive(Debug)]
pub enum ConnectError {
    /// The library was compiled with the `dlopen` feature, and the `libwayland-client.so`
    /// library could not be found at runtime
    NoWaylandLib,
    /// The `XDG_RUNTIME_DIR` variable is not set while it should be
    XdgRuntimeDirNotSet,
    /// Any needed library was found, but the listening socket of the server could not be
    /// found.
    ///
    /// Most of the time, this means that the program was not started from a wayland session.
    NoCompositorListening,
    /// The provided socket name is invalid
    InvalidName,
    /// The FD provided in `WAYLAND_SOCKET` was invalid
    InvalidFd,
}

impl ::std::error::Error for ConnectError {
    fn description(&self) -> &str {
        match *self {
            ConnectError::NoWaylandLib => "Could not find libwayland-client.so.",
            ConnectError::XdgRuntimeDirNotSet => "XDG_RUNTIME_DIR is not set.",
            ConnectError::NoCompositorListening => "Could not find a listening wayland compositor.",
            ConnectError::InvalidName => "The wayland socket name is invalid.",
            ConnectError::InvalidFd => "The FD provided in WAYLAND_SOCKET is invalid.",
        }
    }
}

impl ::std::fmt::Display for ConnectError {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> Result<(), ::std::fmt::Error> {
        f.write_str(::std::error::Error::description(self))
    }
}

/// A connection to a wayland server
///
/// This object both represent the connection to the server, and as such
/// must be kept alive as long as you are connected, and contains the
/// primary `WlDisplay` wayland object, from which you can create all
/// your need objects. The inner `Proxy<WlDisplay>` can be accessed via
/// `Deref`.
pub struct Display {
    pub(crate) inner: Arc<DisplayInner>,
}

impl Display {
    /// Attempt to connect to a wayland server using the contents of the environment variables
    ///
    /// First of all, if the `WAYLAND_SOCKET` environment variable is set, it'll try to interpret
    /// it as a FD number to use
    ///
    /// If the `WAYLAND_DISPLAY` variable is set, it will try to connect to the socket it points
    /// to. Otherwise, it will default to `wayland-0`.
    ///
    /// On success, you are given the `Display` object as well as the main `EventQueue` hosting
    /// the `WlDisplay` wayland object.
    ///
    /// This requires the `XDG_RUNTIME_DIR` variable to be properly set.
    pub fn connect_to_env() -> Result<(Display, EventQueue), ConnectError> {
        if let Ok(txt) = env::var("WAYLAND_SOCKET") {
            // We should connect to the provided WAYLAND_SOCKET
            let fd = txt.parse::<i32>().map_err(|_| ConnectError::InvalidFd)?;
            // set the CLOEXEC flag on this FD
            let flags = fcntl::fcntl(fd, fcntl::FcntlArg::F_GETFD);
            let result = flags
                .map(|f| fcntl::FdFlag::from_bits(f).unwrap() | fcntl::FdFlag::FD_CLOEXEC)
                .and_then(|f| fcntl::fcntl(fd, fcntl::FcntlArg::F_SETFD(f)));
            match result {
                Ok(_) => {
                    // setting the O_CLOEXEC worked
                    unsafe { Display::from_fd(fd) }
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
                .ok_or(ConnectError::XdgRuntimeDirNotSet)?;
            socket_path.push(env::var_os("WAYLAND_DISPLAY").unwrap_or_else(|| "wayland-0".into()));

            let socket = UnixStream::connect(socket_path).map_err(|_| ConnectError::NoCompositorListening)?;
            unsafe { Display::from_fd(socket.into_raw_fd()) }
        }
    }

    /// Attempt to connect to a wayland server socket with given name
    ///
    /// On success, you are given the `Display` object as well as the main `EventQueue` hosting
    /// the `WlDisplay` wayland object.
    ///
    /// This requires the `XDG_RUNTIME_DIR` variable to be properly set.
    pub fn connect_to_name<S: Into<OsString>>(name: S) -> Result<(Display, EventQueue), ConnectError> {
        let mut socket_path = env::var_os("XDG_RUNTIME_DIR")
            .map(Into::<PathBuf>::into)
            .ok_or(ConnectError::XdgRuntimeDirNotSet)?;
        socket_path.push(name.into());

        let socket = UnixStream::connect(socket_path).map_err(|_| ConnectError::NoCompositorListening)?;
        unsafe { Display::from_fd(socket.into_raw_fd()) }
    }

    /// Attempt to use an already connected unix socket on given FD to start a wayland connection
    ///
    /// On success, you are given the `Display` object as well as the main `EventQueue` hosting
    /// the `WlDisplay` wayland object.
    ///
    /// Will take ownership of the FD.
    pub unsafe fn from_fd(fd: RawFd) -> Result<(Display, EventQueue), ConnectError> {
        let (d_inner, evq_inner) = DisplayInner::from_fd(fd)?;
        Ok((Display { inner: d_inner }, EventQueue::new(evq_inner)))
    }

    /// Non-blocking write to the server
    ///
    /// Outgoing messages to the server are buffered by the library for efficiency. This method
    /// flushes the internal buffer to the server socket.
    ///
    /// Will write as many pending requests as possible to the server socket. Never blocks: if not all
    /// requests could be written, will return an io error `WouldBlock`.
    ///
    /// On success returns the number of written requests.
    pub fn flush(&self) -> io::Result<()> {
        self.inner.flush()
    }

    /// Create a new event queue associated with this wayland connection
    pub fn create_event_queue(&self) -> EventQueue {
        let evq_inner = DisplayInner::create_event_queue(&self.inner);
        EventQueue::new(evq_inner)
    }

    #[cfg(feature = "native_lib")]
    /// Create a Display and Event Queue from an external display
    ///
    /// This allows you to interface with an already-existing wayland connection,
    /// for example provided by a GUI toolkit.
    ///
    /// To avoid interferences with the owner of the connection, wayland-client will
    /// create a new event queue and register a wrapper of the `wl_display` to this queue,
    /// then provide them to you. You can then use them as if they came from a direct
    /// wayland connection.
    ///
    /// Note that if you need to retrieve the actual `wl_display` pointer back (rather than
    /// its wrapper), you must use the `get_display_ptr()` method.
    pub unsafe fn from_external_display(display_ptr: *mut wl_display) -> (Display, EventQueue) {
        let (d_inner, evq_inner) = DisplayInner::from_external(display_ptr);
        (Display { inner: d_inner }, EventQueue::new(evq_inner))
    }

    #[cfg(feature = "native_lib")]
    /// Retrieve the `wl_display` pointer
    ///
    /// If this `Display` was created from an external `wl_display`, its `c_ptr()` method will
    /// return a wrapper to the actual display. While this is perfectly good as a `wl_proxy`
    /// pointer, to send requests, this is not the actual `wl_display` and cannot be used as such.
    ///
    /// This method will give you the `wl_display`.
    pub fn get_display_ptr(&self) -> *mut wl_display {
        self.inner.ptr()
    }
}

impl Deref for Display {
    type Target = Proxy<::protocol::wl_display::WlDisplay>;
    fn deref(&self) -> &Proxy<::protocol::wl_display::WlDisplay> {
        self.inner.get_proxy()
    }
}
