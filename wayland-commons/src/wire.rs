//! Types and routines used to manipulate arguments from the wire format

use std::ffi::{CStr, CString};
use std::os::unix::io::RawFd;
use std::ptr;

use nix::{Error as NixError, Result as NixResult};

use smallvec::SmallVec;

// The value of 4 is chosen for the following reasons:
// - almost all messages have 4 arguments or less
// - there are some potentially spammy events that have 3/4 arguments (wl_touch.move has 4 for example)
//
// This brings the size of Message to 11*usize (instead of 4*usize with a regular vec), but eliminates
// almost all allocations that may occur during the processing of messages, both client-side and server-side.
const INLINE_ARGS: usize = 4;

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
    pub destructor: bool,
}

/// Enum of possible argument types as recognized by the wire
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum ArgumentType {
    /// i32
    Int,
    /// u32
    Uint,
    /// fixed point, 1/256 precision
    Fixed,
    /// CString
    Str,
    /// id of a wayland object
    Object,
    /// id of a newly created wayland object
    NewId,
    /// Vec<u8>
    Array,
    /// RawFd
    Fd,
}

/// Enum of possible argument as recognized by the wire, including values
#[derive(Clone, PartialEq, Debug)]
#[allow(clippy::box_collection)]
pub enum Argument {
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
    Str(Box<CString>),
    /// id of a wayland object
    Object(u32),
    /// id of a newly created wayland object
    NewId(u32),
    /// Vec<u8>
    ///
    /// The value is boxed to reduce the stack size of Argument. The performance
    /// impact is negligible as `array` arguments are pretty rare in the protocol.
    Array(Box<Vec<u8>>),
    /// RawFd
    Fd(RawFd),
}

impl Argument {
    /// Retrieve the type of a given argument instance
    pub fn get_type(&self) -> ArgumentType {
        match *self {
            Argument::Int(_) => ArgumentType::Int,
            Argument::Uint(_) => ArgumentType::Uint,
            Argument::Fixed(_) => ArgumentType::Fixed,
            Argument::Str(_) => ArgumentType::Str,
            Argument::Object(_) => ArgumentType::Object,
            Argument::NewId(_) => ArgumentType::NewId,
            Argument::Array(_) => ArgumentType::Array,
            Argument::Fd(_) => ArgumentType::Fd,
        }
    }
}

impl std::fmt::Display for Argument {
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

/// A wire message
#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    /// ID of the object sending this message
    pub sender_id: u32,
    /// Opcode of the message
    pub opcode: u16,
    /// Arguments of the message
    pub args: SmallVec<[Argument; INLINE_ARGS]>,
}

/// Error generated when trying to serialize a message into buffers
#[derive(Debug, Clone)]
pub enum MessageWriteError {
    /// The buffer is too small to hold the message contents
    BufferTooSmall,
    /// The message contains a FD that could not be dup-ed
    DupFdFailed(::nix::Error),
}

impl std::error::Error for MessageWriteError {}

impl std::fmt::Display for MessageWriteError {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> Result<(), ::std::fmt::Error> {
        match *self {
            MessageWriteError::BufferTooSmall => {
                f.write_str("The provided buffer is too small to hold message content.")
            }
            MessageWriteError::DupFdFailed(_) => {
                f.write_str("The message contains a file descriptor that could not be dup()-ed.")
            }
        }
    }
}

/// Error generated when trying to deserialize a message from buffers
#[derive(Debug, Clone)]
pub enum MessageParseError {
    /// The message references a FD but the buffer FD is empty
    MissingFD,
    /// More data is needed to deserialize the message
    MissingData,
    /// The message is malformed and cannot be parsed
    Malformed,
}

impl std::error::Error for MessageParseError {}

impl std::fmt::Display for MessageParseError {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> Result<(), ::std::fmt::Error> {
        match *self {
            MessageParseError::MissingFD => {
                f.write_str("The message references a FD but the buffer FD is empty.")
            }
            MessageParseError::MissingData => {
                f.write_str("More data is needed to deserialize the message")
            }
            MessageParseError::Malformed => {
                f.write_str("The message is malformed and cannot be parsed")
            }
        }
    }
}

impl Message {
    /// Serialize the contents of this message into provided buffers
    ///
    /// Returns the number of elements written in each buffer
    ///
    /// Any serialized Fd will be `dup()`-ed in the process
    pub fn write_to_buffers(
        &self,
        payload: &mut [u32],
        mut fds: &mut [RawFd],
    ) -> Result<(usize, usize), MessageWriteError> {
        let orig_payload_len = payload.len();
        let orig_fds_len = fds.len();
        // Helper function to write a u32 or a RawFd to its buffer
        fn write_buf<T>(u: T, payload: &mut [T]) -> Result<&mut [T], MessageWriteError> {
            if let Some((head, tail)) = payload.split_first_mut() {
                *head = u;
                Ok(tail)
            } else {
                Err(MessageWriteError::BufferTooSmall)
            }
        }

        // Helper function to write byte arrays in payload
        fn write_array_to_payload<'a>(
            array: &[u8],
            payload: &'a mut [u32],
        ) -> Result<&'a mut [u32], MessageWriteError> {
            let array_len = array.len();
            let word_len = array_len / 4 + if array_len % 4 != 0 { 1 } else { 0 };
            // need enough space to store the whole array with padding and a size header
            if payload.len() < 1 + word_len {
                return Err(MessageWriteError::BufferTooSmall);
            }
            // size header
            payload[0] = array_len as u32;
            let (buffer_slice, rest) = payload[1..].split_at_mut(word_len);
            unsafe {
                ptr::copy(array.as_ptr(), buffer_slice.as_mut_ptr() as *mut u8, array_len);
            }
            Ok(rest)
        }

        let free_size = payload.len();
        if free_size < 2 {
            return Err(MessageWriteError::BufferTooSmall);
        }

        let (header, mut payload) = payload.split_at_mut(2);

        // we store all fds we dup-ed in this, which will auto-close
        // them on drop, if any of the `?` early-returns
        let mut pending_fds = FdStore::new();

        // write the contents in the buffer
        for arg in &self.args {
            // Just to make the borrow checker happy
            let old_payload = payload;
            match *arg {
                Argument::Int(i) => payload = write_buf(i as u32, old_payload)?,
                Argument::Uint(u) => payload = write_buf(u, old_payload)?,
                Argument::Fixed(f) => payload = write_buf(f as u32, old_payload)?,
                Argument::Str(ref s) => {
                    payload = write_array_to_payload(s.as_bytes_with_nul(), old_payload)?;
                }
                Argument::Object(o) => payload = write_buf(o, old_payload)?,
                Argument::NewId(n) => payload = write_buf(n, old_payload)?,
                Argument::Array(ref a) => {
                    payload = write_array_to_payload(a, old_payload)?;
                }
                Argument::Fd(fd) => {
                    let old_fds = fds;
                    let dup_fd = dup_fd_cloexec(fd).map_err(MessageWriteError::DupFdFailed)?;
                    pending_fds.push(dup_fd);
                    fds = write_buf(dup_fd, old_fds)?;
                    payload = old_payload;
                }
            }
        }

        // we reached here, all writing was successful
        // no FD needs to be closed
        pending_fds.clear();

        let wrote_size = (free_size - payload.len()) * 4;
        header[0] = self.sender_id;
        header[1] = ((wrote_size as u32) << 16) | u32::from(self.opcode);
        Ok((orig_payload_len - payload.len(), orig_fds_len - fds.len()))
    }

    /// Attempts to parse a single wayland message with the given signature.
    ///
    /// If the buffers contains several messages, only the first one will be parsed,
    /// and the unused tail of the buffers is returned. If a single message was present,
    /// the returned slices should thus be empty.
    ///
    /// Errors if the message is malformed.
    pub fn from_raw<'a, 'b>(
        raw: &'a [u32],
        signature: &[ArgumentType],
        fds: &'b [RawFd],
    ) -> Result<(Message, &'a [u32], &'b [RawFd]), MessageParseError> {
        // helper function to read arrays
        fn read_array_from_payload(
            array_len: usize,
            payload: &[u32],
        ) -> Result<(&[u8], &[u32]), MessageParseError> {
            let word_len = array_len / 4 + if array_len % 4 != 0 { 1 } else { 0 };
            if word_len > payload.len() {
                return Err(MessageParseError::MissingData);
            }
            let (array_contents, rest) = payload.split_at(word_len);
            let array = unsafe {
                ::std::slice::from_raw_parts(array_contents.as_ptr() as *const u8, array_len)
            };
            Ok((array, rest))
        }

        if raw.len() < 2 {
            return Err(MessageParseError::MissingData);
        }

        let sender_id = raw[0];
        let word_2 = raw[1];
        let opcode = (word_2 & 0x0000_FFFF) as u16;
        let len = (word_2 >> 16) as usize / 4;

        if len < 2 || len > raw.len() {
            return Err(MessageParseError::Malformed);
        }

        let (mut payload, rest) = raw.split_at(len);
        payload = &payload[2..];
        let mut fds = fds;

        let arguments = signature
            .iter()
            .map(|argtype| {
                if let ArgumentType::Fd = *argtype {
                    // don't consume input but fd
                    if let Some((&front, tail)) = fds.split_first() {
                        fds = tail;
                        Ok(Argument::Fd(front))
                    } else {
                        Err(MessageParseError::MissingFD)
                    }
                } else if let Some((&front, mut tail)) = payload.split_first() {
                    let arg = match *argtype {
                        ArgumentType::Int => Ok(Argument::Int(front as i32)),
                        ArgumentType::Uint => Ok(Argument::Uint(front)),
                        ArgumentType::Fixed => Ok(Argument::Fixed(front as i32)),
                        ArgumentType::Str => read_array_from_payload(front as usize, tail)
                            .and_then(|(v, rest)| {
                                tail = rest;
                                match CStr::from_bytes_with_nul(v) {
                                    Ok(s) => Ok(Argument::Str(Box::new(s.into()))),
                                    Err(_) => Err(MessageParseError::Malformed),
                                }
                            }),
                        ArgumentType::Object => Ok(Argument::Object(front)),
                        ArgumentType::NewId => Ok(Argument::NewId(front)),
                        ArgumentType::Array => {
                            read_array_from_payload(front as usize, tail).map(|(v, rest)| {
                                tail = rest;
                                Argument::Array(Box::new(v.into()))
                            })
                        }
                        ArgumentType::Fd => unreachable!(),
                    };
                    payload = tail;
                    arg
                } else {
                    Err(MessageParseError::MissingData)
                }
            })
            .collect::<Result<SmallVec<_>, MessageParseError>>()?;

        let msg = Message { sender_id, opcode, args: arguments };
        Ok((msg, rest, fds))
    }
}

/// Duplicate a `RawFd` and set the CLOEXEC flag on the copy
pub fn dup_fd_cloexec(fd: RawFd) -> NixResult<RawFd> {
    use nix::fcntl;
    match fcntl::fcntl(fd, fcntl::FcntlArg::F_DUPFD_CLOEXEC(0)) {
        Ok(newfd) => Ok(newfd),
        Err(NixError::EINVAL) => {
            // F_DUPFD_CLOEXEC is not recognized, kernel too old, fallback
            // to setting CLOEXEC manually
            let newfd = fcntl::fcntl(fd, fcntl::FcntlArg::F_DUPFD(0))?;

            let flags = fcntl::fcntl(newfd, fcntl::FcntlArg::F_GETFD);
            let result = flags
                .map(|f| fcntl::FdFlag::from_bits(f).unwrap() | fcntl::FdFlag::FD_CLOEXEC)
                .and_then(|f| fcntl::fcntl(newfd, fcntl::FcntlArg::F_SETFD(f)));
            match result {
                Ok(_) => {
                    // setting the O_CLOEXEC worked
                    Ok(newfd)
                }
                Err(e) => {
                    // something went wrong in F_GETFD or F_SETFD
                    let _ = ::nix::unistd::close(newfd);
                    Err(e)
                }
            }
        }
        Err(e) => Err(e),
    }
}

/*
 * utility struct that closes every FD it contains on drop
 */

struct FdStore {
    fds: Vec<RawFd>,
}

impl FdStore {
    fn new() -> FdStore {
        FdStore { fds: Vec::new() }
    }
    fn push(&mut self, fd: RawFd) {
        self.fds.push(fd);
    }
    fn clear(&mut self) {
        self.fds.clear();
    }
}

impl Drop for FdStore {
    fn drop(&mut self) {
        use nix::unistd::close;
        for fd in self.fds.drain(..) {
            // not much can be done if we can't close that anyway...
            let _ = close(fd);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use smallvec::smallvec;

    #[test]
    fn into_from_raw_cycle() {
        let mut bytes_buffer = vec![0; 1024];
        let mut fd_buffer = vec![0; 10];

        let msg = Message {
            sender_id: 42,
            opcode: 7,
            args: smallvec![
                Argument::Uint(3),
                Argument::Fixed(-89),
                Argument::Str(Box::new(CString::new(&b"I like trains!"[..]).unwrap())),
                Argument::Array(vec![1, 2, 3, 4, 5, 6, 7, 8, 9].into()),
                Argument::Object(88),
                Argument::NewId(56),
                Argument::Int(-25),
            ],
        };
        // write the message to the buffers
        msg.write_to_buffers(&mut bytes_buffer[..], &mut fd_buffer[..]).unwrap();
        // read them back
        let (rebuilt, _, _) = Message::from_raw(
            &bytes_buffer[..],
            &[
                ArgumentType::Uint,
                ArgumentType::Fixed,
                ArgumentType::Str,
                ArgumentType::Array,
                ArgumentType::Object,
                ArgumentType::NewId,
                ArgumentType::Int,
            ],
            &fd_buffer[..],
        )
        .unwrap();
        assert_eq!(rebuilt, msg);
    }
}
