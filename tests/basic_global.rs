mod helpers;

use helpers::{roundtrip, wayc, ways, TestClient, TestServer};

use wayc::protocol::wl_display::RequestsTrait;
use ways::protocol::wl_compositor::WlCompositor as ServerCompositor;

#[test]
fn simple_global() {
    let mut server = TestServer::new();
    let loop_token = server.event_loop.token();
    server
        .display
        .create_global::<ServerCompositor, _>(&loop_token, 1, |_, _| {});

    let mut client = TestClient::new(&server.socket_name);
    let manager = wayc::GlobalManager::new(&client.display);

    roundtrip(&mut client, &mut server);
    let globals = manager.list();
    assert!(globals.len() == 1);
    assert_eq!(globals[0], (1, "wl_compositor".into(), 1));
}

#[test]
fn multi_versions() {
    let mut server = TestServer::new();
    let loop_token = server.event_loop.token();
    server
        .display
        .create_global::<ServerCompositor, _>(&loop_token, 4, |_, _| {});
    server
        .display
        .create_global::<ServerCompositor, _>(&loop_token, 3, |_, _| {});
    server
        .display
        .create_global::<ServerCompositor, _>(&loop_token, 2, |_, _| {});
    server
        .display
        .create_global::<ServerCompositor, _>(&loop_token, 1, |_, _| {});

    let mut client = TestClient::new(&server.socket_name);
    let manager = wayc::GlobalManager::new(&client.display);

    roundtrip(&mut client, &mut server);
    let globals = manager.list();
    assert!(globals.len() == 4);
    let mut seen = [false; 4];
    for &(_, ref interface, version) in &globals {
        assert!(interface == "wl_compositor");
        seen[version as usize - 1] = true;
    }
    assert_eq!(seen, [true, true, true, true]);
}
