#[macro_use(wayland_env)]
extern crate wayland_client as wayc;
extern crate wayland_server as ways;

mod helpers;

use helpers::{TestClient, TestServer, roundtrip};

use std::sync::atomic::AtomicBool;

static mut INSTANCIATED: bool = false;

mod server_utils {
    use ways::{Client, EventLoop, EventLoopHandle, GlobalHandler};
    use ways::protocol::wl_compositor::WlCompositor;

    struct CompositorHandler;

    impl GlobalHandler<WlCompositor, String> for CompositorHandler {
        fn bind(&mut self, _: &mut EventLoopHandle, _: &Client, _: WlCompositor, txt: &mut String) {
            assert_eq!(txt, "I like trains!");
            unsafe {
                super::INSTANCIATED = true;
            }
        }
    }

    pub fn insert_compositor(event_loop: &mut EventLoop) {
        let hid = event_loop.add_handler(CompositorHandler);
        let _ = event_loop
            .register_global::<WlCompositor, _, CompositorHandler>(hid, 1, "I like trains!".into());
    }
}

mod client_utils {
    use wayc::{EnvHandler, EventQueue};
    use wayc::protocol::wl_registry::WlRegistry;

    wayland_env!(pub ClientEnv,
        compositor: ::wayc::protocol::wl_compositor::WlCompositor
    );

    pub fn insert_handler(event_queue: &mut EventQueue, registry: &WlRegistry) -> usize {
        let hid = event_queue.add_handler(EnvHandler::<ClientEnv>::new());
        event_queue.register::<_, EnvHandler<ClientEnv>>(registry, hid);
        hid
    }

}

#[test]
fn global_user_data() {
    // server setup
    let mut server = TestServer::new();
    self::server_utils::insert_compositor(&mut server.event_loop);

    // client setup
    //
    let mut client = TestClient::new(&server.socket_name);
    let client_registry = client.display.get_registry();
    let client_handler_hid = self::client_utils::insert_handler(&mut client.event_queue, &client_registry);

    // double roundtrip for env init
    //
    roundtrip(&mut client, &mut server);
    roundtrip(&mut client, &mut server);

    // global has now been instancied ?
    unsafe {
        assert_eq!(INSTANCIATED, true);
    }
}
