#[macro_use]
mod helpers;

use helpers::{globals, roundtrip, wayc, ways, DumbClientData, TestClient, TestServer};

use ways::protocol::wl_output::WlOutput as ServerOutput;

use std::os::unix::io::IntoRawFd;
use std::sync::Arc;

fn main() {
    let mut server = TestServer::new();
    server.display.handle().create_global::<ServerData, ServerOutput, _>(2, ());

    let (s1, s2) = ::std::os::unix::net::UnixStream::pair().unwrap();

    let my_client = server.display.handle().insert_client(s1, Arc::new(DumbClientData)).unwrap();

    let fd2 = s2.into_raw_fd();
    ::std::env::set_var("WAYLAND_SOCKET", format!("{fd2}"));

    let mut client = TestClient::new_from_env();

    let mut client_data = ClientHandler::new();

    client.display.get_registry(&client.event_queue.handle(), ());

    roundtrip(&mut client, &mut server, &mut client_data, &mut ServerData).unwrap();
    // check that we connected to the right compositor
    assert!(client_data.globals.list().len() == 1);
    let output = &client_data.globals.list()[0];
    assert_eq!(output.name, 1);
    assert_eq!(output.interface, "wl_output");
    assert_eq!(output.version, 2);

    my_client.kill(
        &server.display.handle(),
        ways::backend::protocol::ProtocolError {
            code: 0,
            object_id: 1,
            object_interface: "wl_display".into(),
            message: "I don't like you!".into(),
        },
    );

    assert!(roundtrip(&mut client, &mut server, &mut client_data, &mut ServerData).is_err());
}

struct ServerData;

server_ignore_impl!(ServerData => [ServerOutput]);
server_ignore_global_impl!(ServerData => [ServerOutput]);

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
