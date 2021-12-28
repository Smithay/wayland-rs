//! Types and utilities for manipulating the Wayland protocol

use std::{ffi::CString, os::unix::io::RawFd};

pub use wayland_sys::common::{wl_argument, wl_interface, wl_message};

/// Describes whether an argument may have a null value.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AllowNull {
    /// Null values are allowed.
    Yes,
    /// Null values are forbidden.
    No,
}

/// Enum of possible argument types as recognized by the wire
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum ArgumentType {
    /// An integer argument. Represented by a [`i32`].
    Int,
    /// An unsigned integer argument. Represented by a [`u32`].
    Uint,
    /// A signed fixed point number with 1/256 precision
    Fixed,
    /// A string. This is represented as a [`CString`] in a message.
    Str(AllowNull),
    /// Id of a wayland object
    Object(AllowNull),
    /// Id of a newly created wayland object
    NewId(AllowNull),
    /// Vec<u8>
    Array(AllowNull),
    /// A file descriptor argument. Represented by a [`RawFd`].
    Fd,
}

impl ArgumentType {
    /// Returns true if the type of the argument is the same.
    pub fn same_type(self, other: Self) -> bool {
        std::mem::discriminant(&self) == std::mem::discriminant(&other)
    }
}

/// Enum of possible argument of the protocol
#[derive(Clone, PartialEq, Eq, Debug)]
#[allow(clippy::box_collection)]
pub enum Argument<Id> {
    /// An integer argument. Represented by a [`i32`].
    Int(i32),
    /// An unsigned integer argument. Represented by a [`u32`].
    Uint(u32),
    /// A signed fixed point number with 1/256 precision
    Fixed(i32),
    /// CString
    ///
    /// The value is boxed to reduce the stack size of Argument. The performance
    /// impact is negligible as `string` arguments are pretty rare in the protocol.
    Str(Box<CString>),
    /// Id of a wayland object
    Object(Id),
    /// Id of a newly created wayland object
    NewId(Id),
    /// Vec<u8>
    ///
    /// The value is boxed to reduce the stack size of Argument. The performance
    /// impact is negligible as `array` arguments are pretty rare in the protocol.
    Array(Box<Vec<u8>>),
    /// A file descriptor argument. Represented by a [`RawFd`].
    Fd(RawFd),
}

impl<Id> Argument<Id> {
    /// Retrieve the type of a given argument instance
    pub fn get_type(&self) -> ArgumentType {
        match *self {
            Argument::Int(_) => ArgumentType::Int,
            Argument::Uint(_) => ArgumentType::Uint,
            Argument::Fixed(_) => ArgumentType::Fixed,
            Argument::Str(_) => ArgumentType::Str(AllowNull::Yes),
            Argument::Object(_) => ArgumentType::Object(AllowNull::Yes),
            Argument::NewId(_) => ArgumentType::NewId(AllowNull::Yes),
            Argument::Array(_) => ArgumentType::Array(AllowNull::Yes),
            Argument::Fd(_) => ArgumentType::Fd,
        }
    }
}

#[cfg(not(tarpaulin_include))]
impl<Id: std::fmt::Display> std::fmt::Display for Argument<Id> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Argument::Int(value) => write!(f, "{}", value),
            Argument::Uint(value) => write!(f, "{}", value),
            Argument::Fixed(value) => write!(f, "{}", value),
            Argument::Str(value) => write!(f, "{:?}", value),
            Argument::Object(value) => write!(f, "{}", value),
            Argument::NewId(value) => write!(f, "{}", value),
            Argument::Array(value) => write!(f, "{:?}", value),
            Argument::Fd(value) => write!(f, "{}", value),
        }
    }
}

/// Description of wayland interface.
///
/// An interface describes the possible requests and events that a wayland client and compositor use to
/// communicate.
#[derive(Debug)]
pub struct Interface {
    /// The name of the interface.
    pub name: &'static str,
    /// The maximum supported version of the interface.
    pub version: u32,
    /// A list that describes every request this interface supports.
    pub requests: &'static [MessageDesc],
    /// A list that describes every event this interface supports.
    pub events: &'static [MessageDesc],
    /// A C representation of this interface that may be used to interoperate with libwayland.
    pub c_ptr: Option<&'static wayland_sys::common::wl_interface>,
}

#[cfg(not(tarpaulin_include))]
impl std::fmt::Display for Interface {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name)
    }
}

/// Wire metadata of a given message
#[derive(Copy, Clone, Debug)]
pub struct MessageDesc {
    /// Name of this message
    pub name: &'static str,
    /// Signature of the message
    pub signature: &'static [ArgumentType],
    /// Minimum required version of the interface
    pub since: u32,
    /// Whether this message is a destructor
    pub is_destructor: bool,
    /// The child interface created from this message.
    ///
    /// In the wayland xml format, this corresponds to the `new_id` type.
    pub child_interface: Option<&'static Interface>,
    /// The interfaces passed into this message as arguments.
    pub arg_interfaces: &'static [&'static Interface],
}

/// Special interface representing an anonymous object
pub static ANONYMOUS_INTERFACE: Interface =
    Interface { name: "<anonymous>", version: 0, requests: &[], events: &[], c_ptr: None };

/// Description of the protocol-level information of an object
#[derive(Copy, Clone, Debug)]
pub struct ObjectInfo {
    /// The protocol ID
    pub id: u32,
    /// The interface
    pub interface: &'static Interface,
    /// The version
    pub version: u32,
}

/// A protocol error
///
/// This kind of error is generated by the server if your client didn't respect
/// the protocol, after which the server will kill your connection.
#[derive(Clone, Debug)]
pub struct ProtocolError {
    /// The error code associated with the error
    ///
    /// It should be interpreted as an instance of the `Error` enum of the
    /// associated interface.
    pub code: u32,
    /// The id of the object that caused the error
    pub object_id: u32,
    /// The interface of the object that caused the error
    pub object_interface: String,
    /// The message sent by the server describing the error
    pub message: String,
}

/// Number of arguments that are stocked inline in a `Message` before allocating
///
/// This is a ad-hoc number trying to reach a good balance between avoiding too many allocations
/// and keeping the stack size of `Message` small.
pub const INLINE_ARGS: usize = 4;

/// Represents a message that has been sent from some object.
#[derive(Debug, Clone, PartialEq)]
pub struct Message<Id> {
    /// The id of the object that sent the message.
    pub sender_id: Id,
    /// The opcode of the message.
    pub opcode: u16,
    /// The arguments of the message.
    pub args: smallvec::SmallVec<[Argument<Id>; INLINE_ARGS]>,
}

impl std::error::Error for ProtocolError {}

#[cfg(not(tarpaulin_include))]
impl std::fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> Result<(), ::std::fmt::Error> {
        write!(
            f,
            "Protocol error {} on object {}@{}: {}",
            self.code, self.object_interface, self.object_id, self.message
        )
    }
}

/// Returns true if the two interfaces are the same.
#[inline]
pub fn same_interface(a: &'static Interface, b: &'static Interface) -> bool {
    std::ptr::eq(a, b) || a.name == b.name
}

pub(crate) fn check_for_signature<Id>(signature: &[ArgumentType], args: &[Argument<Id>]) -> bool {
    if signature.len() != args.len() {
        return false;
    }
    for (typ, arg) in signature.iter().copied().zip(args.iter()) {
        if !arg.get_type().same_type(typ) {
            return false;
        }
    }
    true
}

#[inline]
#[allow(dead_code)]
pub(crate) fn same_interface_or_anonymous(a: &'static Interface, b: &'static Interface) -> bool {
    same_interface(a, b) || same_interface(a, &ANONYMOUS_INTERFACE)
}

/// An enum value in the protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WEnum<T> {
    /// The interpreted value
    Value(T),
    /// The stored value does not match one defined by the protocol file
    Unknown(u32),
}

impl<T: std::convert::TryFrom<u32>> From<u32> for WEnum<T> {
    /// Constructs an enum from the integer format used by the wayland protocol.
    fn from(v: u32) -> WEnum<T> {
        match T::try_from(v) {
            Ok(t) => WEnum::Value(t),
            Err(_) => WEnum::Unknown(v),
        }
    }
}

impl<T: Into<u32>> From<WEnum<T>> for u32 {
    /// Converts an enum into a numerical form used by the wayland protocol.
    fn from(enu: WEnum<T>) -> u32 {
        match enu {
            WEnum::Unknown(u) => u,
            WEnum::Value(t) => t.into(),
        }
    }
}
