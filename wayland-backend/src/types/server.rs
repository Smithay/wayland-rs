use crate::protocol::Interface;

/// Description of a global advertised to some clients.
#[derive(Debug)]
pub struct GlobalInfo {
    /// The interface of the global.
    pub interface: &'static Interface,
    /// The version of the global that is advertised to clients.
    pub version: u32,
    /// Whether the global is disabled.
    pub disabled: bool,
}

/// An error type representing the failure to initialize a backend
#[derive(Debug)]
pub enum InitError {
    /// The wayland system libary could not be loaded
    NoWaylandLib,
    /// Initializaed failed due to an underlying I/O error
    Io(std::io::Error),
}

#[cfg(not(tarpaulin_include))]
impl std::error::Error for InitError {
    fn cause(&self) -> Option<&dyn std::error::Error> {
        match self {
            InitError::Io(ref err) => Some(err),
            InitError::NoWaylandLib => None,
        }
    }
}

#[cfg(not(tarpaulin_include))]
impl std::fmt::Display for InitError {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> Result<(), ::std::fmt::Error> {
        match self {
            InitError::Io(ref err) => std::fmt::Display::fmt(err, f),
            InitError::NoWaylandLib => f.write_str("could not load libwayland-server.so"),
        }
    }
}

/// An error generated when trying to act on an invalid `ObjectId`.
#[derive(Clone, Debug)]
pub struct InvalidId;

impl std::error::Error for InvalidId {}

#[cfg(not(tarpaulin_include))]
impl std::fmt::Display for InvalidId {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> Result<(), ::std::fmt::Error> {
        write!(f, "Invalid Id")
    }
}

/// Describes why a client has been disconnected from the server.
#[derive(Debug)]
pub enum DisconnectReason {
    /// The connection has been closed by the server or client.
    ConnectionClosed,
    /// The server has sent the client a protocol error, terminating the connection.
    ProtocolError(crate::protocol::ProtocolError),
}
