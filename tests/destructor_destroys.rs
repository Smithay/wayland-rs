#[macro_use(wayland_env)]
extern crate wayland_client as wayc;
extern crate wayland_server as ways;

mod helpers;

use helpers::{roundtrip, TestClient, TestServer};

mod server_utils {
    use ways::EventLoop;
    use ways::protocol::wl_compositor::WlCompositor;

    // max supported version: 4
    pub fn insert_compositor(event_loop: &mut EventLoop) {
        let _ = event_loop.register_global::<WlCompositor, ()>(4, |_, _, _, _| {}, ());
    }
}

wayland_env!(pub ClientEnv,
    compositor: wayc::protocol::wl_compositor::WlCompositor
);

#[test]
fn destroy() {
    use wayc::Proxy;
    // Server setup
    //
    let mut server = TestServer::new();
    self::server_utils::insert_compositor(&mut server.event_loop);

    // Client setup
    //
    let mut client = TestClient::new(&server.socket_name);
    let client_registry = client.display.get_registry();
    let client_env_token = wayc::EnvHandler::<ClientEnv>::init(&mut client.event_queue, &client_registry);

    // Some message passing
    //
    roundtrip(&mut client, &mut server);

    // Final asserts
    //
    let state = client.event_queue.state();
    let env = state.get(&client_env_token);
    let surface = env.compositor.create_surface();
    assert!(surface.status() == wayc::Liveness::Alive);
    let msg1 = surface.destroy();
    if let wayc::RequestResult::Destroyed = msg1 {
        panic!("First message should succeed.");
    }
    assert!(surface.status() == wayc::Liveness::Dead);
    let msg2 = surface.destroy();
    if let wayc::RequestResult::Sent(_) = msg2 {
        panic!("First message should fail.");
    }

}

#[test]
fn destroy_implementation_data() {
    use std::rc::Rc;
    use std::cell::Cell;
    // Server setup
    //
    let mut server = TestServer::new();
    self::server_utils::insert_compositor(&mut server.event_loop);

    // Client setup
    //
    let mut client = TestClient::new(&server.socket_name);
    let client_registry = client.display.get_registry();
    let client_env_token = wayc::EnvHandler::<ClientEnv>::init(&mut client.event_queue, &client_registry);

    // Some message passing
    //
    roundtrip(&mut client, &mut server);

    // Utility
    //
    struct IData {
        destroyed: Rc<Cell<bool>>
    }

    impl Drop for IData {
        fn drop(&mut self) {
            self.destroyed.set(true);
        }
    }

    // Final asserts
    //
    let surface = client.event_queue.state().with_value(&client_env_token, |_, env| {
        env.compositor.create_surface()
    });

    let idata = IData { destroyed: Rc::new(Cell::new(false)) };
    let destroyed = idata.destroyed.clone();
    let implem = wayc::protocol::wl_surface::Implementation {
        enter: |_,_,_,_| {},
        leave: |_,_,_,_| {}
    };
    client.event_queue.register(&surface, implem, idata);
    surface.destroy();
    assert!(destroyed.get());
}
