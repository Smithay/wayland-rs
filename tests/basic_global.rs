#[macro_use]
extern crate wayland_client as wayc;
extern crate wayland_server as ways;

mod helpers;

mod server_utils {
    use ways::{Client, EventLoop, EventLoopHandle, GlobalHandler};
    use ways::protocol::wl_compositor::WlCompositor;

    struct CompositorHandler;

    impl GlobalHandler<WlCompositor> for CompositorHandler {
        fn bind(&mut self, evlh: &mut EventLoopHandle, client: &Client, global: WlCompositor) {}
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
    let (mut server_display, mut server_event_loop) = ways::create_display();
    let socket_name = server_display
        .add_socket_auto()
        .expect("Failed to create a server socket.");
    println!("{:?}", socket_name);
    self::server_utils::insert_compositor(&mut server_event_loop, 1);

    // client setup
    //
    let (mut client_display, mut client_event_queue) =
        wayc::connect_to(&socket_name).expect("Failed to connect to server.");
    let client_registry = client_display.get_registry();
    let client_handler_hid = self::client_utils::insert_handler(&mut client_event_queue, &client_registry);

    // message passing
    //
    client_display.flush().unwrap();
    // for some reason, two dispatches are needed
    server_event_loop.dispatch(Some(10)).unwrap();
    server_event_loop.dispatch(Some(10)).unwrap();
    server_display.flush_clients();
    client_event_queue.dispatch().unwrap();

    // result assertions
    //
    let state = client_event_queue.state();
    let env = state.get_handler::<wayc::EnvHandler<ClientEnv>>(client_handler_hid);
    let globals = env.globals();
    assert!(globals.len() == 1);
    assert_eq!(globals[0], (1, "wl_compositor".into(), 1));
}

#[test]
fn multi_versions() {
    // server setup
    //
    let (mut server_display, mut server_event_loop) = ways::create_display();
    let socket_name = server_display
        .add_socket_auto()
        .expect("Failed to create a server socket.");
    println!("{:?}", socket_name);
    self::server_utils::insert_compositor(&mut server_event_loop, 4);
    self::server_utils::insert_compositor(&mut server_event_loop, 2);
    self::server_utils::insert_compositor(&mut server_event_loop, 3);
    self::server_utils::insert_compositor(&mut server_event_loop, 1);

    // client setup
    //
    let (mut client_display, mut client_event_queue) =
        wayc::connect_to(&socket_name).expect("Failed to connect to server.");
    let client_registry = client_display.get_registry();
    let client_handler_hid = self::client_utils::insert_handler(&mut client_event_queue, &client_registry);

    // message passing
    //
    client_display.flush().unwrap();
    // for some reason, two dispatches are needed
    server_event_loop.dispatch(Some(10)).unwrap();
    server_event_loop.dispatch(Some(10)).unwrap();
    server_display.flush_clients();
    client_event_queue.dispatch().unwrap();

    // result assertions
    //
    let state = client_event_queue.state();
    let env = state.get_handler::<wayc::EnvHandler<ClientEnv>>(client_handler_hid);
    let globals = env.globals();
    assert!(globals.len() == 4);
    let mut seen = [false; 4];
    for &(id, ref interface, version) in globals {
        assert!(interface == "wl_compositor");
        seen[version as usize - 1] = true;
    }
    assert_eq!(seen, [true, true, true, true]);
}
