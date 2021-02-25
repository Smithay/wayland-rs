use std::os::unix::io::RawFd;

use crate::wire::ArgumentType;

pub mod client;
pub mod server;

// Description of the protocol-level information of an object
pub struct ObjectInfo {
    /// The protocol ID
    id: u32,
    /// The interface
    interface: &'static str,
    /// The version
    version: u32,
}

/// Enum of possible argument of the protocol
#[derive(Clone, PartialEq, Debug)]
#[allow(clippy::box_vec)]
pub enum Argument<Id: Clone + std::fmt::Debug> {
    /// i32
    Int(i32),
    /// u32
    Uint(u32),
    /// fixed point, 1/256 precision
    Fixed(i32),
    /// CString
    ///
    /// The value is boxed to reduce the stack size of Argument. The performance
    /// impact is negligible as `string` arguments are pretty rare in the protocol.
    Str(Box<String>),
    /// id of a wayland object
    Object(Id),
    /// id of a newly created wayland object
    NewId(Id),
    /// Vec<u8>
    ///
    /// The value is boxed to reduce the stack size of Argument. The performance
    /// impact is negligible as `array` arguments are pretty rare in the protocol.
    Array(Box<Vec<u8>>),
    /// RawFd
    Fd(RawFd),
}

pub struct Interface {
    pub name: &'static str,
    pub version: u32,
    pub requests: &'static [MessageDesc],
    pub events: &'static [MessageDesc],
}

pub struct MessageDesc {
    pub name: &'static str,
    pub since: u32,
    pub is_destructor: bool,
    pub signature: &'static [ArgumentType],
    pub child_interface: Option<&'static Interface>,
}
