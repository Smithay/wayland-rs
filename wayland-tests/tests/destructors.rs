#[macro_use]
mod helpers;

use helpers::{globals, roundtrip, wayc, ways, TestServer};

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

#[test]
fn resource_destructor_request() {
    let mut server = TestServer::new();
    server
        .display
        .handle()
        .create_global::<ServerHandler, ways::protocol::wl_output::WlOutput, _>(3, ());
    let mut server_ddata = ServerHandler { destructor_called: Arc::new(AtomicBool::new(false)) };

    let (_, mut client) = server.add_client();
    let mut client_ddata = ClientHandler::new();

    let registry = client.display.get_registry(&client.event_queue.handle(), ());

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_ddata).unwrap();

    let output = client_ddata
        .globals
        .bind::<wayc::protocol::wl_output::WlOutput, _, _>(
            &client.event_queue.handle(),
            &registry,
            3..4,
            (),
        )
        .unwrap();

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_ddata).unwrap();

    output.release();

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_ddata).unwrap();

    assert!(server_ddata.destructor_called.load(Ordering::Acquire));
}

#[test]
fn resource_destructor_cleanup() {
    let mut server = TestServer::new();
    server
        .display
        .handle()
        .create_global::<ServerHandler, ways::protocol::wl_output::WlOutput, _>(3, ());
    let mut server_ddata = ServerHandler { destructor_called: Arc::new(AtomicBool::new(false)) };

    let (_, mut client) = server.add_client();
    let mut client_ddata = ClientHandler::new();

    let registry = client.display.get_registry(&client.event_queue.handle(), ());

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_ddata).unwrap();

    client_ddata
        .globals
        .bind::<wayc::protocol::wl_output::WlOutput, _, _>(
            &client.event_queue.handle(),
            &registry,
            3..4,
            (),
        )
        .unwrap();

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_ddata).unwrap();

    ::std::mem::drop(client);

    server.answer(&mut server_ddata);

    assert!(server_ddata.destructor_called.load(Ordering::Acquire));
}

#[test]
fn client_destructor_cleanup() {
    let mut server = TestServer::new();
    server
        .display
        .handle()
        .create_global::<ServerHandler, ways::protocol::wl_output::WlOutput, _>(3, ());
    let mut server_ddata = ServerHandler { destructor_called: Arc::new(AtomicBool::new(false)) };

    let destructor_called = Arc::new(AtomicBool::new(false));

    let (_, mut client) =
        server.add_client_with_data(Arc::new(DestructorClientData(destructor_called.clone())));
    let mut client_ddata = ClientHandler::new();

    let registry = client.display.get_registry(&client.event_queue.handle(), ());

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_ddata).unwrap();

    client_ddata
        .globals
        .bind::<wayc::protocol::wl_output::WlOutput, _, _>(
            &client.event_queue.handle(),
            &registry,
            3..4,
            (),
        )
        .unwrap();

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_ddata).unwrap();

    ::std::mem::drop(client);

    server.answer(&mut server_ddata);

    assert!(destructor_called.load(Ordering::Acquire));
}

struct DestructorClientData(Arc<AtomicBool>);

impl ways::backend::ClientData for DestructorClientData {
    fn initialized(&self, _: wayland_backend::server::ClientId) {}

    fn disconnected(
        &self,
        _: wayland_backend::server::ClientId,
        _: wayland_backend::server::DisconnectReason,
    ) {
        self.0.store(true, Ordering::Release)
    }
}

struct ServerHandler {
    destructor_called: Arc<AtomicBool>,
}

struct ServerUData(Arc<AtomicBool>);

impl ways::GlobalDispatch<ways::protocol::wl_output::WlOutput, ()> for ServerHandler {
    fn bind(
        state: &mut Self,
        _: &ways::DisplayHandle,
        _: &ways::Client,
        output: ways::New<ways::protocol::wl_output::WlOutput>,
        _: &(),
        data_init: &mut ways::DataInit<'_, Self>,
    ) {
        data_init.init(output, ServerUData(state.destructor_called.clone()));
    }
}

impl ways::Dispatch<ways::protocol::wl_output::WlOutput, ServerUData> for ServerHandler {
    fn request(
        _: &mut Self,
        _: &ways::Client,
        _: &ways::protocol::wl_output::WlOutput,
        _: ways::protocol::wl_output::Request,
        _: &ServerUData,
        _: &ways::DisplayHandle,
        _: &mut ways::DataInit<'_, Self>,
    ) {
    }

    fn destroyed(
        _: &mut Self,
        _: ways::backend::ClientId,
        _resource: &ways::protocol::wl_output::WlOutput,
        data: &ServerUData,
    ) {
        data.0.store(true, Ordering::Release);
    }
}

struct ClientHandler {
    globals: globals::GlobalList,
}

impl ClientHandler {
    fn new() -> ClientHandler {
        ClientHandler { globals: Default::default() }
    }
}

impl AsMut<globals::GlobalList> for ClientHandler {
    fn as_mut(&mut self) -> &mut globals::GlobalList {
        &mut self.globals
    }
}

wayc::delegate_dispatch!(ClientHandler:
    [wayc::protocol::wl_registry::WlRegistry: ()] => globals::GlobalList
);

client_ignore_impl!(ClientHandler => [
    wayc::protocol::wl_output::WlOutput
]);
