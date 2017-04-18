// This module contains helpers functions and types that
// are not test in themselves, but are used by several tests.

use std::ffi::{OsStr, OsString};

pub struct TestServer {
    pub display: ::ways::Display,
    pub event_loop: ::ways::EventLoop,
    pub socket_name: OsString,
}

impl TestServer {
    pub fn new() -> TestServer {
        let (mut display, event_loop) = ::ways::create_display();
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
        // for some reason, two dispatches are needed
        self.event_loop.dispatch(Some(10)).unwrap();
        self.event_loop.dispatch(Some(10)).unwrap();
        self.display.flush_clients();
    }
}

pub struct TestClient {
    pub display: ::wayc::protocol::wl_display::WlDisplay,
    pub event_queue: ::wayc::EventQueue,
}

impl TestClient {
    pub fn new(socket_name: &OsStr) -> TestClient {
        let (display, event_queue) = ::wayc::connect_to(socket_name).expect("Failed to connect to server.");
        TestClient {
            display: display,
            event_queue: event_queue,
        }
    }
}
