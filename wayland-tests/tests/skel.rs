mod helpers;

use helpers::*;

#[test]
fn skel() {
    // Server setup
    //
    let mut server = TestServer::new();

    // Client setup
    //
    let (_, mut client) = server.add_client();

    // Some message passing
    //
    roundtrip(&mut client, &mut server, &mut ClientHandler, &mut ServerHandler).unwrap();

    // Final asserts
    //
    assert!(true);
}

struct ServerHandler;

struct ClientHandler;