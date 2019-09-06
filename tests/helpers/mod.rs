// This module contains helpers functions and types that
// are not test in themselves, but are used by several tests.

#![allow(dead_code)]

pub extern crate wayland_client as wayc;
pub extern crate wayland_server as ways;

use std::cell::Cell;
use std::ffi::{OsStr, OsString};
use std::io;
use std::os::unix::io::RawFd;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

pub struct TestServer {
    pub display: self::ways::Display,
    pub socket_name: OsString,
}

impl TestServer {
    pub fn new() -> TestServer {
        let mut display = self::ways::Display::new();
        let socket_name = display
            .add_socket_auto()
            .expect("Failed to create a server socket.");

        TestServer {
            display: display,
            socket_name: socket_name,
        }
    }

    pub fn answer(&mut self) {
        self.display.dispatch(Duration::from_millis(10)).unwrap();
        self.display.flush_clients();
        // TODO: find out why native_lib requires two dispatches
        self.display.dispatch(Duration::from_millis(10)).unwrap();
        self.display.flush_clients();
    }
}

pub struct TestClient {
    pub display: Arc<self::wayc::Display>,
    pub display_proxy: self::wayc::Attached<self::wayc::protocol::wl_display::WlDisplay>,
    pub event_queue: self::wayc::EventQueue,
}

impl TestClient {
    pub fn new(socket_name: &OsStr) -> TestClient {
        let display =
            self::wayc::Display::connect_to_name(socket_name).expect("Failed to connect to server.");
        let event_queue = display.create_event_queue();
        let attached = (*display).clone().attach(event_queue.get_token());
        TestClient {
            display: Arc::new(display),
            display_proxy: attached,
            event_queue,
        }
    }

    pub fn new_auto() -> TestClient {
        let display = self::wayc::Display::connect_to_env().expect("Failed to connect to server.");
        let event_queue = display.create_event_queue();
        let attached = (*display).clone().attach(event_queue.get_token());
        TestClient {
            display: Arc::new(display),
            display_proxy: attached,
            event_queue,
        }
    }

    pub unsafe fn from_fd(fd: RawFd) -> TestClient {
        let display = self::wayc::Display::from_fd(fd).unwrap();
        let event_queue = display.create_event_queue();
        let attached = (*display).clone().attach(event_queue.get_token());
        TestClient {
            display: Arc::new(display),
            display_proxy: attached,
            event_queue,
        }
    }
}

pub fn roundtrip(client: &mut TestClient, server: &mut TestServer) -> io::Result<()> {
    // send to the server
    let done = Rc::new(Cell::new(false));
    let done2 = done.clone();
    client
        .display_proxy
        .sync()
        .assign_mono(move |_, _| done2.set(true));
    while !done.get() {
        match client.display.flush() {
            Ok(_) => {}
            Err(e) => {
                if e.kind() != ::std::io::ErrorKind::BrokenPipe {
                    return Err(e);
                }
            }
        }
        ::std::thread::sleep(::std::time::Duration::from_millis(100));
        // make it answer messages
        server.answer();
        ::std::thread::sleep(::std::time::Duration::from_millis(100));
        // dispatch all client-side
        client.event_queue.dispatch_pending(|_, _| {})?;
        let e = client.event_queue.prepare_read().unwrap().read_events();
        // even if read_events returns an error, some messages may need dispatching
        client.event_queue.dispatch_pending(|_, _| {})?;
        e?;
    }
    Ok(())
}
