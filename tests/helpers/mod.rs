// This module contains helpers functions and types that
// are not test in themselves, but are used by several tests.

#![allow(dead_code)]

pub extern crate wayland_client as wayc;
pub extern crate wayland_server as ways;

use std::cell::Cell;
use std::ffi::{OsStr, OsString};
use std::io;
use std::rc::Rc;

pub struct TestServer {
    pub display: self::ways::Display,
    pub event_loop: self::ways::EventLoop,
    pub socket_name: OsString,
}

impl TestServer {
    pub fn new() -> TestServer {
        let (mut display, event_loop) = self::ways::Display::new();
        let socket_name = display
            .add_socket_auto()
            .expect("Failed to create a server socket.");

        TestServer {
            display: display,
            event_loop: event_loop,
            socket_name: socket_name,
        }
    }

    pub fn answer(&mut self) {
        self.event_loop.dispatch(Some(10)).unwrap();
        self.display.flush_clients();
        // TODO: find out why native_lib requires two dispatches
        self.event_loop.dispatch(Some(10)).unwrap();
        self.display.flush_clients();
    }
}

pub struct TestClient {
    pub display: self::wayc::Display,
    pub event_queue: self::wayc::EventQueue,
}

impl TestClient {
    pub fn new(socket_name: &OsStr) -> TestClient {
        let (display, event_queue) =
            self::wayc::Display::connect_to_name(socket_name).expect("Failed to connect to server.");
        TestClient {
            display: display,
            event_queue: event_queue,
        }
    }

    pub fn new_auto() -> TestClient {
        let (display, event_queue) =
            self::wayc::Display::connect_to_env().expect("Failed to connect to server.");
        TestClient {
            display: display,
            event_queue: event_queue,
        }
    }
}

pub fn roundtrip(client: &mut TestClient, server: &mut TestServer) -> io::Result<()> {
    use self::wayc::protocol::wl_display::RequestsTrait;
    // send to the server
    let done = Rc::new(Cell::new(false));
    let done2 = done.clone();
    let token = client.event_queue.get_token();
    client
        .display
        .sync(move |newcb| unsafe { newcb.implement_nonsend(move |_, _| done2.set(true), &token) })
        .unwrap();
    while !done.get() {
        client.display.flush()?;
        ::std::thread::sleep(::std::time::Duration::from_millis(100));
        // make it answer messages
        server.answer();
        ::std::thread::sleep(::std::time::Duration::from_millis(100));
        // dispatch all client-side
        client.event_queue.dispatch_pending()?;
        client.event_queue.prepare_read().unwrap().read_events()?;
        client.event_queue.dispatch_pending()?;
    }
    Ok(())
}
