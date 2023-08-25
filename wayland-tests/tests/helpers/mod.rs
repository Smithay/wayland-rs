// This module contains helpers functions and types that
// are not test in themselves, but are used by several tests.

#![allow(dead_code, unused_macros)]

pub extern crate wayland_client as wayc;
pub extern crate wayland_server as ways;

use std::os::fd::OwnedFd;
use std::os::unix::net::UnixStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use wayland_backend::client::ObjectData;

pub mod globals;

pub struct TestServer<D: 'static> {
    pub display: self::ways::Display<D>,
}

impl<D: 'static> TestServer<D> {
    pub fn new() -> TestServer<D> {
        let display = self::ways::Display::new().unwrap();
        TestServer { display }
    }

    pub fn answer(&mut self, ddata: &mut D) {
        self.display.dispatch_clients(ddata).unwrap();
        self.display.flush_clients().unwrap();
    }

    pub fn add_client_with_data<CD>(
        &mut self,
        data: Arc<dyn ways::backend::ClientData>,
    ) -> (ways::Client, TestClient<CD>) {
        let (server_socket, client_socket) = UnixStream::pair().unwrap();
        let client = self.display.handle().insert_client(server_socket, data).unwrap();
        let test_client = TestClient::new(client_socket);
        (client, test_client)
    }

    pub fn add_client<CD>(&mut self) -> (ways::Client, TestClient<CD>) {
        self.add_client_with_data(Arc::new(DumbClientData))
    }
}

pub struct TestClient<D> {
    pub conn: self::wayc::Connection,
    pub display: self::wayc::protocol::wl_display::WlDisplay,
    pub event_queue: self::wayc::EventQueue<D>,
}

impl<D> TestClient<D> {
    pub fn new(socket: UnixStream) -> TestClient<D> {
        let conn =
            self::wayc::Connection::from_socket(socket).expect("Failed to connect to server.");
        let event_queue = conn.new_event_queue();
        let display = conn.display();
        TestClient { conn, display, event_queue }
    }

    pub fn new_from_env() -> TestClient<D> {
        let conn = self::wayc::Connection::connect_to_env().expect("Failed to connect to server.");
        let event_queue = conn.new_event_queue();
        let display = conn.display();
        TestClient { conn, display, event_queue }
    }
}

pub fn roundtrip<CD: 'static, SD: 'static>(
    client: &mut TestClient<CD>,
    server: &mut TestServer<SD>,
    client_ddata: &mut CD,
    server_ddata: &mut SD,
) -> Result<(), wayc::backend::WaylandError> {
    // send to the server
    let done = Arc::new(AtomicBool::new(false));
    let done2 = done.clone();
    client
        .conn
        .send_request(
            &client.display,
            wayc::protocol::wl_display::Request::Sync {},
            Some(Arc::new(SyncData { done })),
        )
        .unwrap();
    while !done2.load(Ordering::Acquire) {
        match client.conn.flush() {
            Ok(_) => {}
            Err(wayc::backend::WaylandError::Io(e))
                if e.kind() == ::std::io::ErrorKind::BrokenPipe => {}
            Err(e) => return Err(e),
        }
        ::std::thread::sleep(::std::time::Duration::from_millis(100));
        // make it answer messages
        server.answer(server_ddata);
        ::std::thread::sleep(::std::time::Duration::from_millis(100));
        // dispatch all client-side
        client.event_queue.dispatch_pending(client_ddata).unwrap();
        let e = client.conn.prepare_read().map(|guard| guard.read()).unwrap_or(Ok(0));
        // even if read_events returns an error, some messages may need dispatching
        client.event_queue.dispatch_pending(client_ddata).unwrap();
        e?;
    }
    Ok(())
}

struct SyncData {
    done: Arc<AtomicBool>,
}

impl wayc::backend::ObjectData for SyncData {
    fn event(
        self: Arc<Self>,
        _backend: &wayc::backend::Backend,
        _msg: self::wayc::backend::protocol::Message<wayc::backend::ObjectId, OwnedFd>,
    ) -> Option<Arc<dyn ObjectData>> {
        self.done.store(true, Ordering::Release);
        None
    }

    fn destroyed(&self, _: wayc::backend::ObjectId) {}
}

pub struct DumbClientData;

impl ways::backend::ClientData for DumbClientData {
    fn initialized(&self, _: ways::backend::ClientId) {}
    fn disconnected(&self, _: ways::backend::ClientId, _: ways::backend::DisconnectReason) {}
}

macro_rules! client_ignore_impl {
    ($handler:ty => [$($iface:ty),*]) => {
        $(
            impl $crate::helpers::wayc::Dispatch<$iface, ()> for $handler {
                fn event(
                    _: &mut Self,
                    _: &$iface,
                    _: <$iface as $crate::helpers::wayc::Proxy>::Event,
                    _: &(),
                    _: &$crate::helpers::wayc::Connection,
                    _: &$crate::helpers::wayc::QueueHandle<Self>,
                ) {
                }
            }
        )*
    }
}

macro_rules! server_ignore_impl {
    ($handler:ty => [$($iface:ty),*]) => {
        $(
            impl $crate::helpers::ways::Dispatch<$iface, ()> for $handler {
                fn request(
                    _: &mut Self,
                    _: &$crate::helpers::ways::Client,
                    _: &$iface,
                    _: <$iface as $crate::helpers::ways::Resource>::Request,
                    _: &(),
                    _: &$crate::helpers::ways::DisplayHandle,
                    _: &mut $crate::helpers::ways::DataInit<'_, Self>,
                ) {
                }
            }
        )*
    }
}

macro_rules! server_ignore_global_impl {
    ($handler:ty => [$($iface:ty),*]) => {
        $(
            impl $crate::helpers::ways::GlobalDispatch<$iface, ()> for $handler {

                fn bind(
                    _: &mut Self,
                    _: &$crate::helpers::ways::DisplayHandle,
                    _: &$crate::helpers::ways::Client,
                    new_id: $crate::helpers::ways::New<$iface>,
                    _: &(),
                    data_init: &mut $crate::helpers::ways::DataInit<'_, Self>,
                ) {
                    data_init.init(new_id, ());
                }
            }
        )*
    }
}
