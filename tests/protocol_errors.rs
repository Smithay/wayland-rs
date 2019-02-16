mod helpers;

use helpers::{roundtrip, wayc, ways, TestClient, TestServer};

extern crate nix;
extern crate wayland_commons as wc;

use wc::socket::{BufferedSocket, Socket};
use wc::wire::{Argument, Message};

use std::cell::RefCell;
use std::env;
use std::os::unix::io::{FromRawFd, IntoRawFd};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::rc::Rc;

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

#[test]
fn client_receive_error() {
    let mut server = TestServer::new();
    let server_output = Rc::new(RefCell::new(None));
    let my_server_output = server_output.clone();
    server
        .display
        .create_global::<ways::protocol::wl_output::WlOutput, _>(3, move |output, _| {
            *my_server_output.borrow_mut() = Some(output.implement_dummy())
        });

    let mut client = TestClient::new(&server.socket_name);
    let manager = wayc::GlobalManager::new(&client.display);

    roundtrip(&mut client, &mut server).unwrap();

    let output = manager
        .instantiate_exact::<wayc::protocol::wl_output::WlOutput, _>(3, |newp| newp.implement_dummy())
        .unwrap();

    roundtrip(&mut client, &mut server).unwrap();

    // the server sends a protocol error
    server_output
        .borrow()
        .as_ref()
        .unwrap()
        .as_ref()
        .post_error(42, "I don't like you!".into());

    // the error has not yet reached the client
    assert!(client.display.protocol_error().is_none());

    assert!(roundtrip(&mut client, &mut server).is_err());
    let error = client.display.protocol_error().unwrap();
    assert_eq!(error.code, 42);
    assert_eq!(error.object_id, 3);
    assert_eq!(error.object_interface, "wl_output");
    // native lib can't give us the message
    #[cfg(not(feature = "client_native"))]
    {
        assert_eq!(error.message, "I don't like you!");
    }
}
