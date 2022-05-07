#[macro_use]
mod helpers;

use helpers::{roundtrip, wayc, ways, TestServer};

use wayland_protocols::xdg::shell::{client as xs_client, server as xs_server};

#[test]
fn xdg_ping() {
    let mut server = TestServer::new();
    server.display.create_global::<xs_server::xdg_wm_base::XdgWmBase>(1, ());
    let mut server_ddata = ServerHandler { received_pong: false };

    let (_, mut client) = server.add_client();
    let mut client_ddata = ClientHandler::new();

    let registry = client.display.get_registry(&client.event_queue.handle(), ()).unwrap();

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_ddata).unwrap();

    client_ddata
        .globals
        .bind::<xs_client::xdg_wm_base::XdgWmBase, _>(
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

impl ways::GlobalDispatch<xs_server::xdg_wm_base::XdgWmBase> for ServerHandler {
    type GlobalData = ();

    fn bind(
        &mut self,
        handle: &mut ways::DisplayHandle<'_>,
        _: &ways::Client,
        resource: ways::New<xs_server::xdg_wm_base::XdgWmBase>,
        _: &Self::GlobalData,
        data_init: &mut ways::DataInit<'_, Self>,
    ) {
        let wm_base = data_init.init(resource, ());
        wm_base.ping(handle, 42);
    }
}

impl ways::Dispatch<xs_server::xdg_wm_base::XdgWmBase> for ServerHandler {
    type UserData = ();

    fn request(
        &mut self,
        _: &ways::Client,
        _: &xs_server::xdg_wm_base::XdgWmBase,
        request: xs_server::xdg_wm_base::Request,
        _: &Self::UserData,
        _: &mut ways::DisplayHandle<'_>,
        _: &mut ways::DataInit<'_, Self>,
    ) {
        match request {
            xs_server::xdg_wm_base::Request::Pong { serial } => {
                assert_eq!(serial, 42);
                self.received_pong = true;
            }
            _ => unreachable!(),
        }
    }
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
    [wayc::protocol::wl_registry::WlRegistry] => wayc::globals::GlobalList
);

impl wayc::Dispatch<xs_client::xdg_wm_base::XdgWmBase> for ClientHandler {
    type UserData = ();

    fn event(
        &mut self,
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
