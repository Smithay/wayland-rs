#[macro_use]
mod helpers;

use helpers::{globals, roundtrip, wayc, ways, TestServer};
use ways::Resource;

#[test]
fn client_receive_generic_error() {
    let mut server = TestServer::new();
    server
        .display
        .handle()
        .create_global::<ServerHandler, ways::protocol::wl_compositor::WlCompositor, _>(1, ());

    let (s_client, mut client) = server.add_client();

    let mut client_ddata = ClientHandler::new();

    let registry = client.display.get_registry(&client.event_queue.handle(), ());

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut ServerHandler).unwrap();

    // Instantiate the globals
    client_ddata
        .globals
        .bind::<wayc::protocol::wl_compositor::WlCompositor, _, _>(
            &client.event_queue.handle(),
            &registry,
            1..2,
            (),
        )
        .unwrap();

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut ServerHandler).unwrap();

    // the server sends a protocol error
    let compositor = s_client
        .object_from_protocol_id::<ways::protocol::wl_compositor::WlCompositor>(
            &server.display.handle(),
            3,
        )
        .unwrap();
    compositor.post_error(42u32, "I don't like you!");

    // the error has not yet reached the client
    assert!(client.conn.protocol_error().is_none());

    assert!(roundtrip(&mut client, &mut server, &mut client_ddata, &mut ServerHandler).is_err());
    let error = client.conn.protocol_error().unwrap();
    assert_eq!(error.code, 42);
    assert_eq!(error.object_id, 3);
    assert_eq!(error.object_interface, "wl_compositor");
    // native lib can't give us the message
    #[cfg(not(feature = "client_system"))]
    {
        assert_eq!(error.message, "I don't like you!");
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
    wayc::protocol::wl_compositor::WlCompositor
]);

struct ServerHandler;

server_ignore_impl!(ServerHandler => [
    ways::protocol::wl_compositor::WlCompositor
]);
server_ignore_global_impl!(ServerHandler => [
    ways::protocol::wl_compositor::WlCompositor
]);
