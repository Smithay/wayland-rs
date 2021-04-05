use super::Interface;

pub struct GlobalInfo {
    pub interface: &'static Interface,
    pub version: u32,
    pub disabled: bool,
}

/// An error type representing the failure to load libwayland
#[derive(Debug)]
pub enum InitError {
    NoWaylandLib,
    Io(std::io::Error),
}

impl std::error::Error for InitError {
    fn cause(&self) -> Option<&dyn std::error::Error> {
        match self {
            &InitError::Io(ref err) => Some(err),
            &InitError::NoWaylandLib => None,
        }
    }
}

impl std::fmt::Display for InitError {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> Result<(), ::std::fmt::Error> {
        match self {
            &InitError::Io(ref err) => std::fmt::Display::fmt(err, f),
            &InitError::NoWaylandLib => f.write_str("could not load libwayland-server.so"),
        }
    }
}

/// An error generated when trying to act on an invalid `ObjectId`.
#[derive(Clone, Debug)]
pub struct InvalidId;

impl std::error::Error for InvalidId {}

impl std::fmt::Display for InvalidId {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> Result<(), ::std::fmt::Error> {
        write!(f, "Invalid Id")
    }
}

pub enum DisconnectReason {
    ConnectionClosed,
    ProtocolError(crate::protocol::ProtocolError),
}
