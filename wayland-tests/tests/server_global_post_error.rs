#[macro_use]
mod helpers;

use helpers::{globals, roundtrip, wayc, ways, TestServer};

use wayc::{protocol::wl_output::WlOutput as ClientOutput, Proxy};

#[test]
fn global_init_post_error() {
    let mut server = TestServer::new();
    server
        .display
        .handle()
        .create_global::<ServerHandler, ways::protocol::wl_output::WlOutput, _>(3, ());
    let mut server_ddata = ServerHandler;

    let (_, mut client) = server.add_client();
    let mut client_ddata = ClientHandler::new();

    let registry = client.display.get_registry(&client.event_queue.handle(), ());

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_ddata).unwrap();

    // create an outputs
    let client_output = client_ddata
        .globals
        .bind::<wayc::protocol::wl_output::WlOutput, _, _>(
            &client.event_queue.handle(),
            &registry,
            3..4,
            (),
        )
        .unwrap();

    let _ = roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_ddata);

    match client.conn.protocol_error() {
        Some(err) => {
            assert_eq!(err.code, 11);
            assert_eq!(err.object_interface, "wl_output");
            assert_eq!(err.object_id, client_output.id().protocol_id());
        }
        None => panic!("Client did not get protocol error"),
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

client_ignore_impl!(ClientHandler => [ClientOutput]);

struct ServerHandler;

impl ways::GlobalDispatch<ways::protocol::wl_output::WlOutput, ()> for ServerHandler {
    fn bind(
        _state: &mut Self,
        _handle: &ways::DisplayHandle,
        _client: &ways::Client,
        resource: ways::New<ways::protocol::wl_output::WlOutput>,
        _global_data: &(),
        data_init: &mut ways::DataInit<'_, Self>,
    ) {
        data_init.post_error(resource, 11u32, "Server posts error when global is created");
    }
}

impl ways::Dispatch<ways::protocol::wl_output::WlOutput, ()> for ServerHandler {
    fn request(
        _state: &mut Self,
        _client: &ways::Client,
        _resource: &ways::protocol::wl_output::WlOutput,
        _request: <ways::protocol::wl_output::WlOutput as ways::Resource>::Request,
        _data: &(),
        _dhandle: &ways::DisplayHandle,
        _data_init: &mut ways::DataInit<'_, Self>,
    ) {
        unreachable!()
    }
}
