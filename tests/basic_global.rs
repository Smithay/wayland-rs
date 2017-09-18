#[macro_use(wayland_env)]
extern crate wayland_client as wayc;
extern crate wayland_server as ways;

mod helpers;

use helpers::{roundtrip, TestClient, TestServer};

mod server_utils {
    use ways::EventLoop;
    use ways::protocol::wl_compositor::WlCompositor;

    // max supported version: 4
    pub fn insert_compositor(event_loop: &mut EventLoop, v: i32) {
        let _ = event_loop.register_global::<WlCompositor, ()>(v, |_, _, _, _| {}, ());
    }
}

wayland_env!(pub ClientEnv);

#[test]
fn simple_global() {
    // server setup
    //
    let mut server = TestServer::new();
    self::server_utils::insert_compositor(&mut server.event_loop, 1);

    // client setup
    //
    let mut client = TestClient::new(&server.socket_name);
    let client_registry = client.display.get_registry();
    let client_env_token = wayc::EnvHandler::<ClientEnv>::init(&mut client.event_queue, &client_registry);

    // message passing
    //
    roundtrip(&mut client, &mut server);

    // result assertions
    //
    let state = client.event_queue.state();
    let env = state.get(&client_env_token);
    let globals = env.globals();
    assert!(globals.len() == 1);
    assert_eq!(globals[0], (1, "wl_compositor".into(), 1));
}

#[test]
fn multi_versions() {
    // server setup
    //
    let mut server = TestServer::new();
    self::server_utils::insert_compositor(&mut server.event_loop, 4);
    self::server_utils::insert_compositor(&mut server.event_loop, 2);
    self::server_utils::insert_compositor(&mut server.event_loop, 3);
    self::server_utils::insert_compositor(&mut server.event_loop, 1);

    // client setup
    //
    let mut client = TestClient::new(&server.socket_name);
    let client_registry = client.display.get_registry();
    let client_env_token = wayc::EnvHandler::<ClientEnv>::init(&mut client.event_queue, &client_registry);

    // message passing
    //
    roundtrip(&mut client, &mut server);

    // result assertions
    //
    let state = client.event_queue.state();
    let env = state.get(&client_env_token);
    let globals = env.globals();
    assert!(globals.len() == 4);
    let mut seen = [false; 4];
    for &(_, ref interface, version) in globals {
        assert!(interface == "wl_compositor");
        seen[version as usize - 1] = true;
    }
    assert_eq!(seen, [true, true, true, true]);
}
