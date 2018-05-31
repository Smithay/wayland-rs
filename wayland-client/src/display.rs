use std::ffi::OsString;
use std::io;
use std::ops::Deref;
use std::rc::Rc;
use std::sync::Arc;

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
    /// Any needed library was found, but the listening socket of the server could not be
    /// found.
    ///
    /// Most of the time, this means that the program was not started from a wayland session.
    NoCompositorListening,
    /// The provided socket name is invalid
    InvalidName,
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
    /// If the `WAYLAND_DISPLAY` variable is set, it will try to connect to the socket it points
    /// to. Otherwise, it will default to `wayland-0`.
    ///
    /// On success, you are given the `Display` object as well as the main `EventQueue` hosting
    /// the `WlDisplay` wayland object.
    ///
    /// This requires the `XDG_RUNTIME_DIR` variable to be properly set.
    pub fn connect_to_env() -> Result<(Display, EventQueue), ConnectError> {
        let (d_inner, evq_inner) = DisplayInner::connect_to_name(None)?;
        Ok((Display { inner: d_inner }, EventQueue::new(evq_inner)))
    }

    /// Attempt to connect to a wayland server socket with given name
    ///
    /// On success, you are given the `Display` object as well as the main `EventQueue` hosting
    /// the `WlDisplay` wayland object.
    ///
    /// This requires the `XDG_RUNTIME_DIR` variable to be properly set.
    pub fn connect_to_name<S: Into<OsString>>(name: S) -> Result<(Display, EventQueue), ConnectError> {
        let (d_inner, evq_inner) = DisplayInner::connect_to_name(Some(name.into()))?;
        Ok((Display { inner: d_inner }, EventQueue::new(evq_inner)))
    }

    /// Non-blocking write to the server
    ///
    /// Outgoing messages to the server are buffered by the library for efficiency. This method
    /// flushes the internal buffer to the server socket.
    ///
    /// Will write as many pending requests as possible to the server socket. Never blocks: if not all
    /// requests coul be written, will return an io error `WouldBlock`.
    ///
    /// On success returns the number of written requests.
    pub fn flush(&self) -> io::Result<i32> {
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
    pub unsafe fn from_external_display(display_ptr: *mut wl_display) -> (Display, EventQueue) {
        let (d_inner, evq_inner) = DisplayInner::from_external(display_ptr);
        (Display { inner: d_inner }, EventQueue::new(evq_inner))
    }
}

impl Deref for Display {
    type Target = Proxy<::protocol::wl_display::WlDisplay>;
    fn deref(&self) -> &Proxy<::protocol::wl_display::WlDisplay> {
        self.inner.get_proxy()
    }
}
