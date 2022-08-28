#[macro_use]
mod helpers;

use helpers::{globals, roundtrip, wayc, ways, TestServer};

use wayland_protocols::xdg::shell::{client as xs_client, server as xs_server};

#[test]
fn xdg_ping() {
    let mut server = TestServer::new();
    server
        .display
        .handle()
        .create_global::<ServerHandler, xs_server::xdg_wm_base::XdgWmBase, _>(1, ());
    let mut server_ddata = ServerHandler { received_pong: false };

    let (_, mut client) = server.add_client();
    let mut client_ddata = ClientHandler::new();

    let registry = client.display.get_registry(&client.event_queue.handle(), ());

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_ddata).unwrap();

    client_ddata
        .globals
        .bind::<xs_client::xdg_wm_base::XdgWmBase, _, _>(
            &client.event_queue.handle(),
            &registry,
            1..2,
            (),
        )
        .unwrap();

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_ddata).unwrap();
    roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_ddata).unwrap();

    assert!(server_ddata.received_pong);
}

struct ServerHandler {
    received_pong: bool,
}

impl ways::GlobalDispatch<xs_server::xdg_wm_base::XdgWmBase, ()> for ServerHandler {
    fn bind(
        _: &mut Self,
        _: &ways::DisplayHandle,
        _: &ways::Client,
        resource: ways::New<xs_server::xdg_wm_base::XdgWmBase>,
        _: &(),
        data_init: &mut ways::DataInit<'_, Self>,
    ) {
        let wm_base = data_init.init(resource, ());
        wm_base.ping(42);
    }
}

impl ways::Dispatch<xs_server::xdg_wm_base::XdgWmBase, ()> for ServerHandler {
    fn request(
        state: &mut Self,
        _: &ways::Client,
        _: &xs_server::xdg_wm_base::XdgWmBase,
        request: xs_server::xdg_wm_base::Request,
        _: &(),
        _: &ways::DisplayHandle,
        _: &mut ways::DataInit<'_, Self>,
    ) {
        match request {
            xs_server::xdg_wm_base::Request::Pong { serial } => {
                assert_eq!(serial, 42);
                state.received_pong = true;
            }
            _ => unreachable!(),
        }
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

impl wayc::Dispatch<xs_client::xdg_wm_base::XdgWmBase, ()> for ClientHandler {
    fn event(
        _: &mut Self,
        wm_base: &xs_client::xdg_wm_base::XdgWmBase,
        event: xs_client::xdg_wm_base::Event,
        _: &(),
        _: &wayc::Connection,
        _: &wayc::QueueHandle<Self>,
    ) {
        match event {
            xs_client::xdg_wm_base::Event::Ping { serial } => wm_base.pong(serial),
            _ => unreachable!(),
        }
    }
}
