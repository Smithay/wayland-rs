#[cfg(feature = "server_native")]
#[macro_use]
extern crate wayland_sys;

#[macro_use]
mod helpers;

use helpers::{roundtrip, wayc, ways, TestServer};

use ways::protocol::{wl_compositor, wl_output, wl_shm};

use std::sync::Arc;

#[test]
fn global_filter() {
    let mut server = TestServer::new();
    // everyone can see compositor and shm
    server
        .display
        .handle()
        .create_global::<ServerHandler, ways::protocol::wl_compositor::WlCompositor>(1, ());
    server.display.handle().create_global::<ServerHandler, ways::protocol::wl_shm::WlShm>(1, ());
    // only privileged can see output
    let privileged_output = server
        .display
        .handle()
        .create_global::<ServerHandler, ways::protocol::wl_output::WlOutput>(1, ());
    let mut server_ddata = ServerHandler;

    let (_, mut client) = server.add_client_with_data(Arc::new(MyClientData { privileged: false }));
    let mut client_ddata = ClientHandler::new();

    client.display.get_registry(&client.event_queue.handle(), ()).unwrap();

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_ddata).unwrap();

    assert_eq!(client_ddata.globals.list().len(), 2);

    let (_, mut priv_client) =
        server.add_client_with_data(Arc::new(MyClientData { privileged: true }));
    let mut priv_client_ddata = ClientHandler::new();

    priv_client.display.get_registry(&priv_client.event_queue.handle(), ()).unwrap();

    roundtrip(&mut priv_client, &mut server, &mut priv_client_ddata, &mut server_ddata).unwrap();

    assert_eq!(priv_client_ddata.globals.list().len(), 3);

    // now destroy the privileged global
    // only privileged clients will receive the destroy event
    // if a regular client received it, it would panic as the server destroyed an
    // unknown global

    server.display.handle().remove_global(privileged_output);

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_ddata).unwrap();
    roundtrip(&mut priv_client, &mut server, &mut priv_client_ddata, &mut server_ddata).unwrap();

    assert_eq!(client_ddata.globals.list().len(), 2);
    assert_eq!(priv_client_ddata.globals.list().len(), 2);
}

#[test]
fn global_filter_try_force() {
    let mut server = TestServer::new();
    // only privileged can see output
    server
        .display
        .handle()
        .create_global::<ServerHandler, ways::protocol::wl_output::WlOutput>(1, ());
    let mut server_ddata = ServerHandler;

    // normal client that cannot bind the privileged global
    let (_, mut client) = server.add_client_with_data(Arc::new(MyClientData { privileged: false }));
    let mut client_ddata = ClientHandler::new();

    // privileged client that can
    let (_, mut priv_client) =
        server.add_client_with_data(Arc::new(MyClientData { privileged: true }));
    let mut priv_client_ddata = ClientHandler::new();

    // privileged client can bind it

    let priv_registry =
        priv_client.display.get_registry(&priv_client.event_queue.handle(), ()).unwrap();
    priv_registry
        .bind::<wayc::protocol::wl_output::WlOutput, _, _>(
            1,
            1,
            &priv_client.event_queue.handle(),
            (),
        )
        .unwrap();
    roundtrip(&mut priv_client, &mut server, &mut priv_client_ddata, &mut server_ddata).unwrap();

    // unprivileged client cannot
    let registry = client.display.get_registry(&client.event_queue.handle(), ()).unwrap();
    registry
        .bind::<wayc::protocol::wl_output::WlOutput, _, _>(1, 1, &client.event_queue.handle(), ())
        .unwrap();

    assert!(roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_ddata).is_err());
}

struct ClientHandler {
    globals: wayc::globals::GlobalList,
}

impl ClientHandler {
    fn new() -> ClientHandler {
        ClientHandler { globals: Default::default() }
    }
}

impl AsMut<wayc::globals::GlobalList> for ClientHandler {
    fn as_mut(&mut self) -> &mut wayc::globals::GlobalList {
        &mut self.globals
    }
}

wayc::delegate_dispatch!(ClientHandler:
    [wayc::protocol::wl_registry::WlRegistry: ()] => wayc::globals::GlobalList
);

client_ignore_impl!(ClientHandler => [
    wayc::protocol::wl_compositor::WlCompositor,
    wayc::protocol::wl_shm::WlShm,
    wayc::protocol::wl_output::WlOutput
]);

struct ServerHandler;

impl ways::GlobalDispatch<wl_compositor::WlCompositor> for ServerHandler {
    type GlobalData = ();
    fn bind(
        &mut self,
        _: &ways::DisplayHandle,
        _: &ways::Client,
        resource: ways::New<wl_compositor::WlCompositor>,
        _: &Self::GlobalData,
        data_init: &mut ways::DataInit<'_, Self>,
    ) {
        data_init.init(resource, ());
    }

    fn can_view(_: ways::Client, _: &Self::GlobalData) -> bool {
        true
    }
}

impl ways::GlobalDispatch<wl_shm::WlShm> for ServerHandler {
    type GlobalData = ();
    fn bind(
        &mut self,
        _: &ways::DisplayHandle,
        _: &ways::Client,
        resource: ways::New<wl_shm::WlShm>,
        _: &Self::GlobalData,
        data_init: &mut ways::DataInit<'_, Self>,
    ) {
        data_init.init(resource, ());
    }

    fn can_view(_: ways::Client, _: &Self::GlobalData) -> bool {
        true
    }
}

impl ways::GlobalDispatch<wl_output::WlOutput> for ServerHandler {
    type GlobalData = ();
    fn bind(
        &mut self,
        _: &ways::DisplayHandle,
        _: &ways::Client,
        resource: ways::New<wl_output::WlOutput>,
        _: &Self::GlobalData,
        data_init: &mut ways::DataInit<'_, Self>,
    ) {
        data_init.init(resource, ());
    }

    fn can_view(client: ways::Client, _: &Self::GlobalData) -> bool {
        client.get_data::<MyClientData>().unwrap().privileged
    }
}

struct MyClientData {
    privileged: bool,
}

impl ways::backend::ClientData<ServerHandler> for MyClientData {
    fn initialized(&self, _: wayland_backend::server::ClientId) {}
    fn disconnected(
        &self,
        _: wayland_backend::server::ClientId,
        _: wayland_backend::server::DisconnectReason,
    ) {
    }
}

server_ignore_impl!(ServerHandler => [
    ways::protocol::wl_compositor::WlCompositor,
    ways::protocol::wl_shm::WlShm,
    ways::protocol::wl_output::WlOutput
]);
