#[macro_use]
mod helpers;

use helpers::{roundtrip, wayc, ways, DumbClientData, TestClient, TestServer};

use ways::protocol::wl_output::WlOutput as ServerOutput;

static SOCKET_NAME: &str = "wayland-rs-test-client-connect-to-env";

fn main() {
    let mut server = TestServer::new();
    server.display.handle().create_global::<ServerData, ServerOutput>(1, ());

    // client fails to connect if environment is not set
    ::std::env::remove_var("WAYLAND_DISPLAY");
    assert!(wayc::Connection::connect_to_env().is_err());

    // setup a listening server
    let listening = ways::socket::ListeningSocket::bind(&SOCKET_NAME).unwrap();

    ::std::env::set_var("WAYLAND_DISPLAY", &SOCKET_NAME);

    // connect the client
    let mut client = TestClient::new_from_env();
    let mut globals = wayc::globals::GlobalList::new();
    client.display.get_registry(&client.event_queue.handle(), ()).unwrap();

    // setup server-side
    let client_stream = listening.accept().unwrap().unwrap();
    server.display.insert_client(client_stream, std::sync::Arc::new(DumbClientData)).unwrap();

    roundtrip(&mut client, &mut server, &mut globals, &mut ServerData).unwrap();
    // check that we connected to the right compositor
    assert!(globals.list().len() == 1);
    let output = &globals.list()[0];
    assert_eq!(output.name, 1);
    assert_eq!(output.interface, "wl_output");
    assert_eq!(output.version, 1);
}

struct ServerData;

server_ignore_impl!(ServerData => [ServerOutput]);
server_ignore_global_impl!(ServerData => [ServerOutput]);
