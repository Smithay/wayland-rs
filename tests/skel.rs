extern crate wayland_client as wayc;
extern crate wayland_server as ways;

mod helpers;

use helpers::{TestClient, TestServer, roundtrip};

#[test]
fn skel() {
    // Server setup
    //
    let mut server = TestServer::new();

    // Client setup
    //
    let mut client = TestClient::new(&server.socket_name);

    // Some message passing
    //
    roundtrip(&mut client, &mut server);

    // Final asserts
    //
    assert!(true);
}
