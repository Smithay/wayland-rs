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
    pub event_loop: self::ways::calloop::EventLoop<()>,
    pub socket_name: OsString,
}

impl TestServer {
    pub fn new() -> TestServer {
        let event_loop = self::ways::calloop::EventLoop::<()>::new().unwrap();
        let mut display = self::ways::Display::new(event_loop.handle());
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
        self.event_loop
            .dispatch(Some(Duration::from_millis(10)), &mut ())
            .unwrap();
        self.display.flush_clients();
        // TODO: find out why native_lib requires two dispatches
        self.event_loop
            .dispatch(Some(Duration::from_millis(10)), &mut ())
            .unwrap();
        self.display.flush_clients();
    }
}

pub struct TestClient {
    pub display: Arc<self::wayc::Display>,
    pub event_queue: self::wayc::EventQueue,
}

impl TestClient {
    pub fn new(socket_name: &OsStr) -> TestClient {
        let (display, event_queue) =
            self::wayc::Display::connect_to_name(socket_name).expect("Failed to connect to server.");
        TestClient {
            display: Arc::new(display),
            event_queue: event_queue,
        }
    }

    pub fn new_auto() -> TestClient {
        let (display, event_queue) =
            self::wayc::Display::connect_to_env().expect("Failed to connect to server.");
        TestClient {
            display: Arc::new(display),
            event_queue: event_queue,
        }
    }

    pub unsafe fn from_fd(fd: RawFd) -> TestClient {
        let (display, event_queue) = self::wayc::Display::from_fd(fd).unwrap();
        TestClient {
            display: Arc::new(display),
            event_queue: event_queue,
        }
    }
}

pub fn roundtrip(client: &mut TestClient, server: &mut TestServer) -> io::Result<()> {
    // send to the server
    let done = Rc::new(Cell::new(false));
    let done2 = done.clone();
    client
        .display
        .sync(move |newcb| newcb.implement_closure(move |_, _| done2.set(true), ()))
        .unwrap();
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
        client.event_queue.dispatch_pending()?;
        let e = client.event_queue.prepare_read().unwrap().read_events();
        // even if read_events returns an error, some messages may need dispatching
        client.event_queue.dispatch_pending()?;
        e?;
    }
    Ok(())
}
