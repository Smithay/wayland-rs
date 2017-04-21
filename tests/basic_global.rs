#[macro_use(wayland_env)]
extern crate wayland_client as wayc;
extern crate wayland_server as ways;

mod helpers;

use helpers::{TestClient, TestServer, roundtrip};

mod server_utils {
    use ways::{Client, EventLoop, EventLoopHandle, GlobalHandler};
    use ways::protocol::wl_compositor::WlCompositor;

    struct CompositorHandler;

    impl GlobalHandler<WlCompositor> for CompositorHandler {
        fn bind(&mut self, _: &mut EventLoopHandle, _: &Client, _: WlCompositor) {}
    }

    // max supported version: 4
    pub fn insert_compositor(event_loop: &mut EventLoop, v: i32) {
        let hid = event_loop.add_handler(CompositorHandler);
        let _ = event_loop.register_global::<WlCompositor, CompositorHandler>(hid, v);
    }
}

wayland_env!(ClientEnv);

mod client_utils {
    use super::ClientEnv;
    use wayc::{EnvHandler, EventQueue};
    use wayc::protocol::wl_registry::WlRegistry;

    pub fn insert_handler(event_queue: &mut EventQueue, registry: &WlRegistry) -> usize {
        let hid = event_queue.add_handler(EnvHandler::<ClientEnv>::new());
        event_queue.register::<_, EnvHandler<ClientEnv>>(registry, hid);
        hid
    }
}

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
    let client_handler_hid = self::client_utils::insert_handler(&mut client.event_queue, &client_registry);

    // message passing
    //
    roundtrip(&mut client, &mut server);

    // result assertions
    //
    let state = client.event_queue.state();
    let env = state.get_handler::<wayc::EnvHandler<ClientEnv>>(client_handler_hid);
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
    let client_handler_hid = self::client_utils::insert_handler(&mut client.event_queue, &client_registry);

    // message passing
    //
    client.display.flush().unwrap();
    server.answer();
    client.event_queue.dispatch().unwrap();

    // result assertions
    //
    let state = client.event_queue.state();
    let env = state.get_handler::<wayc::EnvHandler<ClientEnv>>(client_handler_hid);
    let globals = env.globals();
    assert!(globals.len() == 4);
    let mut seen = [false; 4];
    for &(_, ref interface, version) in globals {
        assert!(interface == "wl_compositor");
        seen[version as usize - 1] = true;
    }
    assert_eq!(seen, [true, true, true, true]);
}
