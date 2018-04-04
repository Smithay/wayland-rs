mod helpers;

use helpers::{roundtrip, TestClient, TestServer};

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
