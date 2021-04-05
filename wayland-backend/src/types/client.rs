/// An error type representing the failure to load libwayland
#[derive(Debug)]
pub struct NoWaylandLib;

impl std::error::Error for NoWaylandLib {}

impl std::fmt::Display for NoWaylandLib {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> Result<(), ::std::fmt::Error> {
        f.write_str("could not load libwayland-client.so")
    }
}

/// An error that can occur when using a Wayland connection
#[derive(Debug)]
pub enum WaylandError {
    /// The connection encountered an IO error
    Io(std::io::Error),
    /// The connection encountered a protocol error
    Protocol(crate::protocol::ProtocolError),
}

impl std::error::Error for WaylandError {
    fn cause(&self) -> Option<&dyn std::error::Error> {
        match self {
            WaylandError::Io(e) => Some(e),
            WaylandError::Protocol(e) => Some(e),
        }
    }
}

impl std::fmt::Display for WaylandError {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> Result<(), ::std::fmt::Error> {
        match self {
            WaylandError::Io(e) => write!(f, "Io error: {}", e),
            WaylandError::Protocol(e) => std::fmt::Display::fmt(e, f),
        }
    }
}

impl Clone for WaylandError {
    fn clone(&self) -> WaylandError {
        match self {
            WaylandError::Protocol(e) => WaylandError::Protocol(e.clone()),
            WaylandError::Io(e) => {
                if let Some(code) = e.raw_os_error() {
                    WaylandError::Io(std::io::Error::from_raw_os_error(code))
                } else {
                    WaylandError::Io(std::io::Error::new(e.kind(), ""))
                }
            }
        }
    }
}

impl From<crate::protocol::ProtocolError> for WaylandError {
    fn from(err: crate::protocol::ProtocolError) -> WaylandError {
        WaylandError::Protocol(err)
    }
}

impl From<std::io::Error> for WaylandError {
    fn from(err: std::io::Error) -> WaylandError {
        WaylandError::Io(err)
    }
}

/// An error generated when trying to act on an invalid `ObjectId`.
#[derive(Clone, Debug)]
pub struct InvalidId;

impl std::error::Error for InvalidId {}

impl std::fmt::Display for InvalidId {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> Result<(), ::std::fmt::Error> {
        write!(f, "Invalid ObjectId")
    }
}
