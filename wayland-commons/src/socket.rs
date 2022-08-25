//! Wayland socket manipulation

use std::io::{IoSlice, IoSliceMut};
use std::os::unix::io::{AsRawFd, FromRawFd, IntoRawFd, RawFd};

use nix::{sys::socket, Result as NixResult};

use crate::wire::{ArgumentType, Message, MessageParseError, MessageWriteError};

/// Maximum number of FD that can be sent in a single socket message
pub const MAX_FDS_OUT: usize = 28;
/// Maximum number of bytes that can be sent in a single socket message
pub const MAX_BYTES_OUT: usize = 4096;

/*
 * Socket
 */

/// A wayland socket
#[derive(Debug)]
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
        let flags = socket::MsgFlags::MSG_DONTWAIT | socket::MsgFlags::MSG_NOSIGNAL;
        let iov = [IoSlice::new(bytes)];

        if !fds.is_empty() {
            let cmsgs = [socket::ControlMessage::ScmRights(fds)];
            socket::sendmsg::<()>(self.fd, &iov, &cmsgs, flags, None)?;
        } else {
            socket::sendmsg::<()>(self.fd, &iov, &[], flags, None)?;
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
        let mut cmsg = cmsg_space!([RawFd; MAX_FDS_OUT]);
        let mut iov = [IoSliceMut::new(buffer)];

        let msg = socket::recvmsg::<()>(
            self.fd,
            &mut iov[..],
            Some(&mut cmsg),
            socket::MsgFlags::MSG_DONTWAIT
                | socket::MsgFlags::MSG_CMSG_CLOEXEC
                | socket::MsgFlags::MSG_NOSIGNAL,
        )?;

        let mut fd_count = 0;
        let received_fds = msg.cmsgs().flat_map(|cmsg| match cmsg {
            socket::ControlMessageOwned::ScmRights(s) => s,
            _ => Vec::new(),
        });
        for (fd, place) in received_fds.zip(fds.iter_mut()) {
            fd_count += 1;
            *place = fd;
        }
        Ok((msg.bytes, fd_count))
    }

    /// Retrieve the current value of the requested [socket::GetSockOpt]
    pub fn opt<O: socket::GetSockOpt>(&self, opt: O) -> NixResult<O::Val> {
        socket::getsockopt(self.fd, opt)
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

impl Drop for Socket {
    fn drop(&mut self) {
        let _ = ::nix::unistd::close(self.fd);
    }
}

/*
 * BufferedSocket
 */

/// An adapter around a raw Socket that directly handles buffering and
/// conversion from/to wayland messages
#[derive(Debug)]
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
            socket,
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

    /// Retrieve ownership of the underlying Socket
    ///
    /// Any leftover content in the internal buffers will be lost
    pub fn into_socket(self) -> Socket {
        self.socket
    }

    /// Flush the contents of the outgoing buffer into the socket
    pub fn flush(&mut self) -> NixResult<()> {
        {
            let words = self.out_data.get_contents();
            if words.is_empty() {
                return Ok(());
            }
            let bytes = unsafe {
                ::std::slice::from_raw_parts(words.as_ptr() as *const u8, words.len() * 4)
            };
            let fds = self.out_fds.get_contents();
            self.socket.send_msg(bytes, fds)?;
            for &fd in fds {
                // once the fds are sent, we can close them
                let _ = ::nix::unistd::close(fd);
            }
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
                return Err(::nix::Error::E2BIG);
            }
        }
        Ok(())
    }

    /// Try to fill the incoming buffers of this socket, to prepare
    /// a new round of parsing.
    pub fn fill_incoming_buffers(&mut self) -> NixResult<()> {
        // clear the buffers if they have no content
        if !self.in_data.has_content() {
            self.in_data.clear();
        }
        if !self.in_fds.has_content() {
            self.in_fds.clear();
        }
        // receive a message
        let (in_bytes, in_fds) = {
            let words = self.in_data.get_writable_storage();
            let bytes = unsafe {
                ::std::slice::from_raw_parts_mut(words.as_ptr() as *mut u8, words.len() * 4)
            };
            let fds = self.in_fds.get_writable_storage();
            self.socket.rcv_msg(bytes, fds)?
        };
        if in_bytes == 0 {
            // the other end of the socket was closed
            return Err(::nix::Error::EPIPE);
        }
        // advance the storage
        self.in_data.advance(in_bytes / 4 + if in_bytes % 4 > 0 { 1 } else { 0 });
        self.in_fds.advance(in_fds);
        Ok(())
    }

    /// Read and deserialize a single message from the incoming buffers socket
    ///
    /// This method requires one closure that given an object id and an opcode,
    /// must provide the signature of the associated request/event, in the form of
    /// a `&'static [ArgumentType]`. If it returns `None`, meaning that
    /// the couple object/opcode does not exist, an error will be returned.
    ///
    /// There are 3 possibilities of return value:
    ///
    /// - `Ok(Ok(msg))`: no error occurred, this is the message
    /// - `Ok(Err(e))`: either a malformed message was encountered or we need more data,
    ///    in the latter case you need to try calling `fill_incoming_buffers()`.
    /// - `Err(e)`: an I/O error occurred reading from the socked, details are in `e`
    ///   (this can be a "wouldblock" error, which just means that no message is available
    ///   to read)
    pub fn read_one_message<F>(&mut self, mut signature: F) -> Result<Message, MessageParseError>
    where
        F: FnMut(u32, u16) -> Option<&'static [ArgumentType]>,
    {
        let (msg, read_data, read_fd) = {
            let data = self.in_data.get_contents();
            let fds = self.in_fds.get_contents();
            if data.len() < 2 {
                return Err(MessageParseError::MissingData);
            }
            let object_id = data[0];
            let opcode = (data[1] & 0x0000_FFFF) as u16;
            if let Some(sig) = signature(object_id, opcode) {
                match Message::from_raw(data, sig, fds) {
                    Ok((msg, rest_data, rest_fds)) => {
                        (msg, data.len() - rest_data.len(), fds.len() - rest_fds.len())
                    }
                    // TODO: gracefully handle wayland messages split across unix messages ?
                    Err(e) => return Err(e),
                }
            } else {
                // no signature found ?
                return Err(MessageParseError::Malformed);
            }
        };

        self.in_data.offset(read_data);
        self.in_fds.offset(read_fd);

        Ok(msg)
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
    /// In both cases of early stopping, the remaining unused data will be left
    /// in the buffers, and will start to be processed at the next call of this
    /// method.
    ///
    /// There are 3 possibilities of return value:
    ///
    /// - `Ok(Ok(n))`: no error occurred, `n` messages where processed
    /// - `Ok(Err(MessageParseError::Malformed))`: a malformed message was encountered
    ///   (this is a protocol error and is supposed to be fatal to the connection).
    /// - `Err(e)`: an I/O error occurred reading from the socked, details are in `e`
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
        // message parsing
        let mut dispatched = 0;

        loop {
            let mut err = None;
            // first parse any leftover messages
            loop {
                match self.read_one_message(&mut signature) {
                    Ok(msg) => {
                        let keep_going = callback(msg);
                        dispatched += 1;
                        if !keep_going {
                            break;
                        }
                    }
                    Err(e) => {
                        err = Some(e);
                        break;
                    }
                }
            }

            // copy back any leftover content to the front of the buffer
            self.in_data.move_to_front();
            self.in_fds.move_to_front();

            if let Some(MessageParseError::Malformed) = err {
                // early stop here
                return Ok(Err(MessageParseError::Malformed));
            }

            if err.is_none() && self.in_data.has_content() {
                // we stopped reading without error while there is content? That means
                // the user requested an early stopping
                return Ok(Ok(dispatched));
            }

            // now, try to get more data
            match self.fill_incoming_buffers() {
                Ok(()) => (),
                Err(e @ ::nix::Error::EAGAIN) => {
                    // stop looping, returning Ok() or EAGAIN depending on whether messages
                    // were dispatched
                    if dispatched == 0 {
                        return Err(e);
                    } else {
                        break;
                    }
                }
                Err(e) => return Err(e),
            }
        }

        Ok(Ok(dispatched))
    }
}

/*
 * Buffer
 */
#[derive(Debug)]
struct Buffer<T: Copy> {
    storage: Vec<T>,
    occupied: usize,
    offset: usize,
}

impl<T: Copy + Default> Buffer<T> {
    fn new(size: usize) -> Buffer<T> {
        Buffer { storage: vec![T::default(); size], occupied: 0, offset: 0 }
    }

    /// Check if this buffer has content to read
    fn has_content(&self) -> bool {
        self.occupied > self.offset
    }

    /// Advance the internal counter of occupied space
    fn advance(&mut self, bytes: usize) {
        self.occupied += bytes;
    }

    /// Advance the read offset of current occupied space
    fn offset(&mut self, bytes: usize) {
        self.offset += bytes;
    }

    /// Clears the contents of the buffer
    ///
    /// This only sets the counter of occupied space back to zero,
    /// allowing previous content to be overwritten.
    fn clear(&mut self) {
        self.occupied = 0;
        self.offset = 0;
    }

    /// Get the current contents of the occupied space of the buffer
    fn get_contents(&self) -> &[T] {
        &self.storage[(self.offset)..(self.occupied)]
    }

    /// Get mutable access to the unoccupied space of the buffer
    fn get_writable_storage(&mut self) -> &mut [T] {
        &mut self.storage[(self.occupied)..]
    }

    /// Move the unread contents of the buffer to the front, to ensure
    /// maximal write space availability
    fn move_to_front(&mut self) {
        unsafe {
            ::std::ptr::copy(
                &self.storage[self.offset] as *const T,
                &mut self.storage[0] as *mut T,
                self.occupied - self.offset,
            );
        }
        self.occupied -= self.offset;
        self.offset = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wire::{Argument, ArgumentType, Message};

    use std::ffi::CString;

    use smallvec::smallvec;

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
            args: smallvec![
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
                args: smallvec![
                    Argument::Int(42),
                    Argument::Str(Box::new(CString::new(&b"I like trains"[..]).unwrap())),
                ],
            },
            Message {
                sender_id: 42,
                opcode: 1,
                args: smallvec![
                    Argument::Fd(1), // stdin
                    Argument::Fd(0), // stdout
                ],
            },
            Message {
                sender_id: 42,
                opcode: 2,
                args: smallvec![
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

    #[test]
    fn parse_with_string_len_multiple_of_4() {
        let msg = Message {
            sender_id: 2,
            opcode: 0,
            args: smallvec![
                Argument::Uint(18),
                Argument::Str(Box::new(CString::new(&b"wl_shell"[..]).unwrap())),
                Argument::Uint(1),
            ],
        };

        let (client, server) = ::std::os::unix::net::UnixStream::pair().unwrap();
        let mut client = BufferedSocket::new(unsafe { Socket::from_raw_fd(client.into_raw_fd()) });
        let mut server = BufferedSocket::new(unsafe { Socket::from_raw_fd(server.into_raw_fd()) });

        client.write_message(&msg).unwrap();
        client.flush().unwrap();

        static SIGNATURE: &'static [ArgumentType] =
            &[ArgumentType::Uint, ArgumentType::Str, ArgumentType::Uint];

        let ret = server
            .read_messages(
                |sender_id, opcode| {
                    if sender_id == 2 && opcode == 0 {
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
}
