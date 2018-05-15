//! Wayland socket manipulation

use std::os::unix::io::{AsRawFd, FromRawFd, IntoRawFd, RawFd};

use nix::Result as NixResult;
use nix::sys::socket;
use nix::sys::uio;

use wire::{ArgumentType, Message, MessageParseError, MessageWriteError};

/// Maximum number of FD that can be sent in a single socket message
pub const MAX_FDS_OUT: usize = 28;
/// Maximum number of bytes that can be sent in a single socket message
pub const MAX_BYTES_OUT: usize = 4096;

/*
 * Socket
 */

/// A wayland socket
pub struct Socket {
    fd: RawFd,
}

impl Socket {
    /// Send a single message to the socket
    ///
    /// A single socket message can contain several wayland messages
    ///
    /// The `fds` slice should not be longer than `MAX_FDS_OUT`, and the `bytes`
    /// slice should not be longer than `MAX_BYTES_OUT` otherwise the receiving
    /// end may lose some data.
    pub fn send_msg(&self, bytes: &[u8], fds: &[RawFd]) -> NixResult<()> {
        let iov = [uio::IoVec::from_slice(bytes)];
        if fds.len() > 0 {
            let cmsgs = [socket::ControlMessage::ScmRights(fds)];
            socket::sendmsg(self.fd, &iov, &cmsgs, socket::MsgFlags::MSG_DONTWAIT, None)?;
        } else {
            socket::sendmsg(self.fd, &iov, &[], socket::MsgFlags::MSG_DONTWAIT, None)?;
        };
        Ok(())
    }

    /// Receive a single message from the socket
    ///
    /// Return the number of bytes received and the number of Fds received.
    ///
    /// Errors with `WouldBlock` is no message is available.
    ///
    /// A single socket message can contain several wayland messages.
    ///
    /// The `buffer` slice should be at least `MAX_BYTES_OUT` long and the `fds`
    /// slice `MAX_FDS_OUT` long, otherwise some data of the received message may
    /// be lost.
    pub fn rcv_msg(&self, buffer: &mut [u8], fds: &mut [RawFd]) -> NixResult<(usize, usize)> {
        let mut cmsg = socket::CmsgSpace::<[RawFd; MAX_FDS_OUT]>::new();
        let iov = [uio::IoVec::from_mut_slice(buffer)];

        let msg = socket::recvmsg(self.fd, &iov[..], Some(&mut cmsg), socket::MsgFlags::MSG_DONTWAIT)?;

        let mut fd_count = 0;
        let received_fds = msg.cmsgs().flat_map(|cmsg| {
            match cmsg {
                socket::ControlMessage::ScmRights(s) => s,
                _ => &[],
            }.iter()
        });
        for (fd, place) in received_fds.zip(fds.iter_mut()) {
            fd_count += 1;
            *place = *fd;
        }
        Ok((msg.bytes, fd_count))
    }
}

impl FromRawFd for Socket {
    unsafe fn from_raw_fd(fd: RawFd) -> Socket {
        Socket { fd }
    }
}

impl AsRawFd for Socket {
    fn as_raw_fd(&self) -> RawFd {
        self.fd
    }
}

impl IntoRawFd for Socket {
    fn into_raw_fd(self) -> RawFd {
        self.fd
    }
}

/*
 * BufferedSocket
 */

/// An adapter around a raw Socket that directly handles buffering and
/// conversion from/to wayland messages
pub struct BufferedSocket {
    socket: Socket,
    in_data: Buffer<u32>,
    in_fds: Buffer<RawFd>,
    out_data: Buffer<u32>,
    out_fds: Buffer<RawFd>,
}

impl BufferedSocket {
    /// Wrap a Socket into a Buffered Socket
    pub fn new(socket: Socket) -> BufferedSocket {
        BufferedSocket {
            socket: socket,
            in_data: Buffer::new(2 * MAX_BYTES_OUT / 4), // Incoming buffers are twice as big in order to be
            in_fds: Buffer::new(2 * MAX_FDS_OUT),        // able to store leftover data if needed
            out_data: Buffer::new(MAX_BYTES_OUT / 4),
            out_fds: Buffer::new(MAX_FDS_OUT),
        }
    }

    /// Get direct access to the underlying socket
    pub fn get_socket(&mut self) -> &mut Socket {
        &mut self.socket
    }

    /// Retreive ownership of the underlying Socket
    ///
    /// Any leftover content in the internal buffers will be lost
    pub fn into_socket(self) -> Socket {
        self.socket
    }

    /// Flush the contents of the outgoing buffer into the socket
    pub fn flush(&mut self) -> NixResult<()> {
        {
            let words = self.out_data.get_contents();
            let bytes = unsafe { ::std::slice::from_raw_parts(words.as_ptr() as *const u8, words.len() * 4) };
            let fds = self.out_fds.get_contents();
            self.socket.send_msg(bytes, fds)?;
        }
        self.out_data.clear();
        self.out_fds.clear();
        Ok(())
    }

    // internal method
    //
    // attempts to write a message in the internal out buffers,
    // returns true if successful
    //
    // if false is returned, it means there is not enough space
    // in the buffer
    fn attempt_write_message(&mut self, msg: &Message) -> NixResult<bool> {
        match msg.write_to_buffers(
            self.out_data.get_writable_storage(),
            self.out_fds.get_writable_storage(),
        ) {
            Ok((bytes_out, fds_out)) => {
                self.out_data.advance(bytes_out);
                self.out_fds.advance(fds_out);
                Ok(true)
            }
            Err(MessageWriteError::BufferTooSmall) => Ok(false),
            Err(MessageWriteError::DupFdFailed(e)) => Err(e),
        }
    }

    /// Write a message to the outgoing buffer
    ///
    /// This method may flush the internal buffer if necessary (if it is full).
    ///
    /// If the message is too big to fit in the buffer, the error `Error::Sys(E2BIG)`
    /// will be returned.
    pub fn write_message(&mut self, msg: &Message) -> NixResult<()> {
        if !self.attempt_write_message(msg)? {
            // the attempt failed, there is not enough space in the buffer
            // we need to flush it
            self.flush()?;
            if !self.attempt_write_message(msg)? {
                // If this fails again, this means the message is too big
                // to be transmitted at all
                return Err(::nix::Error::Sys(::nix::errno::Errno::E2BIG));
            }
        }
        Ok(())
    }

    /// Read and deserialize messages from the socket
    ///
    /// This method requires two closures:
    ///
    /// - The first one, given an object id and an opcode, must provide
    ///   the signature of the associated request/event, in the form of
    ///   a `&'static [ArgumentType]`. If it returns `None`, meaning that
    ///   the couple object/opcode does not exist, the parsing will be
    ///   prematurely interrupted and this method will return a
    ///   `MessageParseError::Malformed` error.
    /// - The second closure is charged to process the parsed message. If it
    ///   returns `false`, the iteration will be prematurely stopped.
    ///
    /// In both cases of early stopping, the remaining unsued data will be left
    /// in the buffers, and will start to be processed at the next call of this
    /// method.
    ///
    /// There are 3 possibilities of return value:
    ///
    /// - `Ok(Ok(n))`: no error occured, `n` messages where processed
    /// - `Ok(Err(MessageParseError::Malformed))`: a malformed message was encountered
    ///   (this is a protocol error and is supposed to be fatal to the connection).
    /// - `Err(e)`: an I/O error occured reading from the socked, details are in `e`
    ///   (this can be a "wouldblock" error, which just means that no message is available
    ///   to read)
    pub fn read_messages<F1, F2>(
        &mut self,
        mut signature: F1,
        mut callback: F2,
    ) -> NixResult<Result<usize, MessageParseError>>
    where
        F1: FnMut(u32, u16) -> Option<&'static [ArgumentType]>,
        F2: FnMut(Message) -> bool,
    {
        // first, receive some data from the socket
        let (in_bytes, in_fds) = {
            let words = self.in_data.get_writable_storage();
            let bytes =
                unsafe { ::std::slice::from_raw_parts_mut(words.as_ptr() as *mut u8, words.len() * 4) };
            let fds = self.in_fds.get_writable_storage();
            self.socket.rcv_msg(bytes, fds)?
        };
        self.in_data
            .advance(in_bytes / 4 + if in_bytes % 4 > 0 { 1 } else { 0 });
        self.in_fds.advance(in_fds);

        // message parsing
        let mut dispatched = 0;
        let mut result = Ok(0);
        let (rest_data_ptr, rest_data_len, rest_fds_ptr, rest_fds_len) = {
            let mut data = self.in_data.get_contents();
            let mut fds = self.in_fds.get_contents();

            loop {
                if data.len() < 2 {
                    break;
                }
                let object_id = data[0];
                let opcode = (data[1] & 0x0000FFFF) as u16;
                if let Some(sig) = signature(object_id, opcode) {
                    match Message::from_raw(data, sig, fds) {
                        Ok((msg, rest_data, rest_fds)) => {
                            let ret = callback(msg);
                            data = rest_data;
                            fds = rest_fds;
                            dispatched += 1;
                            if !ret {
                                break;
                            }
                        }
                        Err(e @ MessageParseError::Malformed) => {
                            result = Err(e);
                            break;
                        }
                        Err(_) => {
                            // missing data or fd, we can't progress until next socket message
                            break;
                        }
                    }
                } else {
                    // no signature found ?
                    result = Err(MessageParseError::Malformed);
                    break;
                }
            }
            (data.as_ptr(), data.len(), fds.as_ptr(), fds.len())
        };

        // copy back any leftover content to the front of the buffer
        self.in_data.clear();
        self.in_fds.clear();
        if rest_data_len > 0 {
            unsafe {
                ::std::ptr::copy(
                    rest_data_ptr,
                    self.in_data.get_writable_storage().as_mut_ptr(),
                    rest_data_len,
                );
            }
            self.in_data.set_occupied(rest_data_len);
        }
        if rest_fds_len > 0 {
            unsafe {
                ::std::ptr::copy(
                    rest_fds_ptr,
                    self.in_fds.get_writable_storage().as_mut_ptr(),
                    rest_fds_len,
                );
            }
            self.in_fds.set_occupied(rest_fds_len);
        }
        Ok(result.map(|_| dispatched))
    }
}

/*
 * Buffer
 */

struct Buffer<T: Copy> {
    storage: Vec<T>,
    occupied: usize,
}

impl<T: Copy + Default> Buffer<T> {
    fn new(size: usize) -> Buffer<T> {
        Buffer {
            storage: vec![T::default(); size],
            occupied: 0,
        }
    }

    /// Advance the internal counter of occupied space
    fn advance(&mut self, bytes: usize) {
        self.occupied += bytes;
    }

    /// Sets the internal counter of occupied space
    fn set_occupied(&mut self, bytes: usize) {
        self.occupied = bytes;
    }

    /// Clears the contents of the buffer
    ///
    /// This only sets the counter of occupied space back to zero,
    /// allowing previous content to be overwritten.
    fn clear(&mut self) {
        self.occupied = 0;
    }

    /// Get the current contents of the occupied space of the buffer
    fn get_contents(&self) -> &[T] {
        &self.storage[..(self.occupied)]
    }

    /// Get mutable access to the unoccupied space of the buffer
    fn get_writable_storage(&mut self) -> &mut [T] {
        &mut self.storage[(self.occupied)..]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wire::{Argument, ArgumentType, Message};

    use std::ffi::CString;

    fn same_file(a: RawFd, b: RawFd) -> bool {
        let stat1 = ::nix::sys::stat::fstat(a).unwrap();
        let stat2 = ::nix::sys::stat::fstat(b).unwrap();
        stat1.st_dev == stat2.st_dev && stat1.st_ino == stat2.st_ino
    }

    // check if two messages are equal
    //
    // if arguments contain FDs, check that the fd point to
    // the same file, rather than are the same number.
    fn assert_eq_msgs(msg1: &Message, msg2: &Message) {
        assert_eq!(msg1.sender_id, msg2.sender_id);
        assert_eq!(msg1.opcode, msg2.opcode);
        assert_eq!(msg1.args.len(), msg2.args.len());
        for (arg1, arg2) in msg1.args.iter().zip(msg2.args.iter()) {
            if let (&Argument::Fd(fd1), &Argument::Fd(fd2)) = (arg1, arg2) {
                assert!(same_file(fd1, fd2));
            } else {
                assert_eq!(arg1, arg2);
            }
        }
    }

    #[test]
    fn write_read_cycle() {
        let msg = Message {
            sender_id: 42,
            opcode: 7,
            args: vec![
                Argument::Uint(3),
                Argument::Fixed(-89),
                Argument::Str(CString::new(&b"I like trains!"[..]).unwrap()),
                Argument::Array(vec![1, 2, 3, 4, 5, 6, 7, 8, 9]),
                Argument::Object(88),
                Argument::NewId(56),
                Argument::Int(-25),
            ],
        };

        let (client, server) = ::std::os::unix::net::UnixStream::pair().unwrap();
        let mut client = BufferedSocket::new(unsafe { Socket::from_raw_fd(client.into_raw_fd()) });
        let mut server = BufferedSocket::new(unsafe { Socket::from_raw_fd(server.into_raw_fd()) });

        client.write_message(&msg).unwrap();
        client.flush().unwrap();

        static SIGNATURE: &'static [ArgumentType] = &[
            ArgumentType::Uint,
            ArgumentType::Fixed,
            ArgumentType::Str,
            ArgumentType::Array,
            ArgumentType::Object,
            ArgumentType::NewId,
            ArgumentType::Int,
        ];

        let ret = server
            .read_messages(
                |sender_id, opcode| {
                    if sender_id == 42 && opcode == 7 {
                        Some(SIGNATURE)
                    } else {
                        None
                    }
                },
                |message| {
                    assert_eq_msgs(&message, &msg);
                    true
                },
            )
            .unwrap()
            .unwrap();

        assert_eq!(ret, 1);
    }

    #[test]
    fn write_read_cycle_fd() {
        let msg = Message {
            sender_id: 42,
            opcode: 7,
            args: vec![
                Argument::Fd(1), // stdin
                Argument::Fd(0), // stdout
            ],
        };

        let (client, server) = ::std::os::unix::net::UnixStream::pair().unwrap();
        let mut client = BufferedSocket::new(unsafe { Socket::from_raw_fd(client.into_raw_fd()) });
        let mut server = BufferedSocket::new(unsafe { Socket::from_raw_fd(server.into_raw_fd()) });

        client.write_message(&msg).unwrap();
        client.flush().unwrap();

        static SIGNATURE: &'static [ArgumentType] = &[ArgumentType::Fd, ArgumentType::Fd];

        let ret = server
            .read_messages(
                |sender_id, opcode| {
                    if sender_id == 42 && opcode == 7 {
                        Some(SIGNATURE)
                    } else {
                        None
                    }
                },
                |message| {
                    assert_eq_msgs(&message, &msg);
                    true
                },
            )
            .unwrap()
            .unwrap();

        assert_eq!(ret, 1);
    }

    #[test]
    fn write_read_cycle_multiple() {
        let messages = [
            Message {
                sender_id: 42,
                opcode: 0,
                args: vec![
                    Argument::Int(42),
                    Argument::Str(CString::new(&b"I like trains"[..]).unwrap()),
                ],
            },
            Message {
                sender_id: 42,
                opcode: 1,
                args: vec![
                    Argument::Fd(1), // stdin
                    Argument::Fd(0), // stdout
                ],
            },
            Message {
                sender_id: 42,
                opcode: 2,
                args: vec![
                    Argument::Uint(3),
                    Argument::Fd(2), // stderr
                ],
            },
        ];

        static SIGNATURES: &'static [&'static [ArgumentType]] = &[
            &[ArgumentType::Int, ArgumentType::Str],
            &[ArgumentType::Fd, ArgumentType::Fd],
            &[ArgumentType::Uint, ArgumentType::Fd],
        ];

        let (client, server) = ::std::os::unix::net::UnixStream::pair().unwrap();
        let mut client = BufferedSocket::new(unsafe { Socket::from_raw_fd(client.into_raw_fd()) });
        let mut server = BufferedSocket::new(unsafe { Socket::from_raw_fd(server.into_raw_fd()) });

        for msg in &messages {
            client.write_message(msg).unwrap();
        }
        client.flush().unwrap();

        let mut recv_msgs = Vec::new();
        let ret = server
            .read_messages(
                |sender_id, opcode| {
                    if sender_id == 42 {
                        Some(SIGNATURES[opcode as usize])
                    } else {
                        None
                    }
                },
                |message| {
                    recv_msgs.push(message);
                    true
                },
            )
            .unwrap()
            .unwrap();

        assert_eq!(ret, 3);
        assert_eq!(recv_msgs.len(), 3);
        for (msg1, msg2) in messages.iter().zip(recv_msgs.iter()) {
            assert_eq_msgs(msg1, msg2);
        }
    }
}
