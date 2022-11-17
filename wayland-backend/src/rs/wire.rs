//! Types and routines used to manipulate arguments from the wire format

use std::os::unix::io::{FromRawFd, RawFd};
use std::ptr;
use std::{ffi::CStr, os::unix::prelude::AsRawFd};

use io_lifetimes::{BorrowedFd, OwnedFd};

use crate::protocol::{Argument, ArgumentType, Message};

use smallvec::SmallVec;

/// Error generated when trying to serialize a message into buffers
#[derive(Debug)]
pub enum MessageWriteError {
    /// The buffer is too small to hold the message contents
    BufferTooSmall,
    /// The message contains a FD that could not be dup-ed
    DupFdFailed(std::io::Error),
}

impl std::error::Error for MessageWriteError {}

impl std::fmt::Display for MessageWriteError {
    #[cfg_attr(coverage, no_coverage)]
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> Result<(), ::std::fmt::Error> {
        match self {
            Self::BufferTooSmall => {
                f.write_str("The provided buffer is too small to hold message content.")
            }
            Self::DupFdFailed(e) => {
                write!(
                    f,
                    "The message contains a file descriptor that could not be dup()-ed ({}).",
                    e
                )
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
    #[cfg_attr(coverage, no_coverage)]
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> Result<(), ::std::fmt::Error> {
        match *self {
            Self::MissingFD => {
                f.write_str("The message references a FD but the buffer FD is empty.")
            }
            Self::MissingData => f.write_str("More data is needed to deserialize the message"),
            Self::Malformed => f.write_str("The message is malformed and cannot be parsed"),
        }
    }
}

/// Serialize the contents of this message into provided buffers
///
/// Returns the number of elements written in each buffer
///
/// Any serialized Fd will be `dup()`-ed in the process
pub fn write_to_buffers(
    msg: &Message<u32, RawFd>,
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
        let word_len = array_len / 4 + usize::from(array_len % 4 != 0);
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

    let mut pending_fds = Vec::new();

    // write the contents in the buffer
    for arg in &msg.args {
        // Just to make the borrow checker happy
        let old_payload = payload;
        match *arg {
            Argument::Int(i) => payload = write_buf(i as u32, old_payload)?,
            Argument::Uint(u) => payload = write_buf(u, old_payload)?,
            Argument::Fixed(f) => payload = write_buf(f as u32, old_payload)?,
            Argument::Str(Some(ref s)) => {
                payload = write_array_to_payload(s.as_bytes_with_nul(), old_payload)?;
            }
            Argument::Str(None) => {
                payload = write_array_to_payload(&[], old_payload)?;
            }
            Argument::Object(o) => payload = write_buf(o, old_payload)?,
            Argument::NewId(n) => payload = write_buf(n, old_payload)?,
            Argument::Array(ref a) => {
                payload = write_array_to_payload(a, old_payload)?;
            }
            Argument::Fd(fd) => {
                let old_fds = fds;
                let dup_fd = unsafe { BorrowedFd::borrow_raw(fd) }
                    .try_clone_to_owned()
                    .map_err(MessageWriteError::DupFdFailed)?;
                let raw_dup_fd = dup_fd.as_raw_fd();
                pending_fds.push(dup_fd);
                fds = write_buf(raw_dup_fd, old_fds)?;
                payload = old_payload;
            }
        }
    }

    // if we reached here, the whole message was written successfully, we can drop the pending_fds they
    // don't need to be closed, sendmsg will take ownership of them
    for fd in pending_fds {
        std::mem::forget(fd);
    }

    let wrote_size = (free_size - payload.len()) * 4;
    header[0] = msg.sender_id;
    header[1] = ((wrote_size as u32) << 16) | u32::from(msg.opcode);
    Ok((orig_payload_len - payload.len(), orig_fds_len - fds.len()))
}

/// Attempts to parse a single wayland message with the given signature.
///
/// If the buffers contains several messages, only the first one will be parsed,
/// and the unused tail of the buffers is returned. If a single message was present,
/// the returned slices should thus be empty.
///
/// Errors if the message is malformed.
#[allow(clippy::type_complexity)]
pub fn parse_message<'a, 'b>(
    raw: &'a [u32],
    signature: &[ArgumentType],
    fds: &'b [RawFd],
) -> Result<(Message<u32, OwnedFd>, &'a [u32], &'b [RawFd]), MessageParseError> {
    // helper function to read arrays
    fn read_array_from_payload(
        array_len: usize,
        payload: &[u32],
    ) -> Result<(&[u8], &[u32]), MessageParseError> {
        let word_len = array_len / 4 + usize::from(array_len % 4 != 0);
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

    if len < 2 {
        return Err(MessageParseError::Malformed);
    } else if len > raw.len() {
        return Err(MessageParseError::MissingData);
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
                    Ok(Argument::Fd(unsafe { OwnedFd::from_raw_fd(front) }))
                } else {
                    Err(MessageParseError::MissingFD)
                }
            } else if let Some((&front, mut tail)) = payload.split_first() {
                let arg = match *argtype {
                    ArgumentType::Int => Ok(Argument::Int(front as i32)),
                    ArgumentType::Uint => Ok(Argument::Uint(front)),
                    ArgumentType::Fixed => Ok(Argument::Fixed(front as i32)),
                    ArgumentType::Str(_) => {
                        read_array_from_payload(front as usize, tail).and_then(|(v, rest)| {
                            tail = rest;
                            if !v.is_empty() {
                                match CStr::from_bytes_with_nul(v) {
                                    Ok(s) => Ok(Argument::Str(Some(Box::new(s.into())))),
                                    Err(_) => Err(MessageParseError::Malformed),
                                }
                            } else {
                                Ok(Argument::Str(None))
                            }
                        })
                    }
                    ArgumentType::Object(_) => Ok(Argument::Object(front)),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::AllowNull;
    use smallvec::smallvec;
    use std::{ffi::CString, os::unix::io::IntoRawFd};

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
                Argument::Str(Some(Box::new(CString::new(&b"I like trains!"[..]).unwrap()))),
                Argument::Array(vec![1, 2, 3, 4, 5, 6, 7, 8, 9].into()),
                Argument::Object(88),
                Argument::NewId(56),
                Argument::Int(-25),
            ],
        };
        // write the message to the buffers
        write_to_buffers(&msg, &mut bytes_buffer[..], &mut fd_buffer[..]).unwrap();
        // read them back
        let (rebuilt, _, _) = parse_message(
            &bytes_buffer[..],
            &[
                ArgumentType::Uint,
                ArgumentType::Fixed,
                ArgumentType::Str(AllowNull::No),
                ArgumentType::Array,
                ArgumentType::Object(AllowNull::No),
                ArgumentType::NewId,
                ArgumentType::Int,
            ],
            &fd_buffer[..],
        )
        .unwrap();
        assert_eq!(rebuilt.map_fd(IntoRawFd::into_raw_fd), msg);
    }
}
