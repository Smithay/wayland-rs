mod helpers;

use helpers::TestServer;

extern crate nix;
extern crate wayland_commons as wc;

use wc::socket::{BufferedSocket, Socket};
use wc::wire::{Argument, Message};

use std::env;
use std::os::unix::io::{FromRawFd, IntoRawFd};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

#[test]
fn client_wrong_id() {
    let mut server = TestServer::new();

    let mut socket: PathBuf = env::var_os("XDG_RUNTIME_DIR").unwrap().into();
    socket.push(&server.socket_name);
    let socket = UnixStream::connect(socket).unwrap();

    let mut socket = BufferedSocket::new(unsafe { Socket::from_raw_fd(socket.into_raw_fd()) });
    socket
        .write_message(&Message {
            sender_id: 1, // wl_display
            opcode: 1,    // wl_registry
            args: vec![
                Argument::NewId(3), // should be 2
            ],
        })
        .unwrap();
    socket.flush().unwrap();

    server.answer();

    // server should have killed us due to the error
    assert_eq!(socket.flush(), Err(nix::Error::Sys(nix::errno::Errno::EPIPE)));
}

#[test]
fn client_wrong_opcode() {
    let mut server = TestServer::new();

    let mut socket: PathBuf = env::var_os("XDG_RUNTIME_DIR").unwrap().into();
    socket.push(&server.socket_name);
    let socket = UnixStream::connect(socket).unwrap();

    let mut socket = BufferedSocket::new(unsafe { Socket::from_raw_fd(socket.into_raw_fd()) });
    socket
        .write_message(&Message {
            sender_id: 1, // wl_display
            opcode: 42,   // inexistant
            args: vec![],
        })
        .unwrap();
    socket.flush().unwrap();

    server.answer();

    // server should have killed us due to the error
    assert_eq!(socket.flush(), Err(nix::Error::Sys(nix::errno::Errno::EPIPE)));
}

#[test]
fn client_wrong_sender() {
    let mut server = TestServer::new();

    let mut socket: PathBuf = env::var_os("XDG_RUNTIME_DIR").unwrap().into();
    socket.push(&server.socket_name);
    let socket = UnixStream::connect(socket).unwrap();

    let mut socket = BufferedSocket::new(unsafe { Socket::from_raw_fd(socket.into_raw_fd()) });
    socket
        .write_message(&Message {
            sender_id: 54, // wl_display
            opcode: 0,     // inexistant
            args: vec![],
        })
        .unwrap();
    socket.flush().unwrap();

    server.answer();

    // server should have killed us due to the error
    assert_eq!(socket.flush(), Err(nix::Error::Sys(nix::errno::Errno::EPIPE)));
}
