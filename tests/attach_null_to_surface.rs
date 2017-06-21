#[macro_use]
extern crate wayland_server as ways;
#[macro_use]
extern crate wayland_client as wayc;

mod helpers;

use helpers::{TestClient, TestServer, roundtrip};

mod server_utils {
    use ways::{Client, EventLoop, EventLoopHandle, GlobalHandler, Init};
    use ways::protocol::{wl_buffer, wl_compositor, wl_surface};

    pub struct CompositorHandler {
        hid: Option<usize>,
        pub got_buffer: bool,
    }

    impl CompositorHandler {
        fn new() -> CompositorHandler {
            CompositorHandler {
                hid: None,
                got_buffer: false,
            }
        }
    }

    impl Init for CompositorHandler {
        fn init(&mut self, _: &mut EventLoopHandle, index: usize) {
            self.hid = Some(index)
        }
    }

    impl GlobalHandler<wl_compositor::WlCompositor> for CompositorHandler {
        fn bind(&mut self, evlh: &mut EventLoopHandle, _: &Client, comp: wl_compositor::WlCompositor) {
            let hid = self.hid.expect("CompositorHandler was not initialized!");
            evlh.register::<_, CompositorHandler>(&comp, hid);
        }
    }

    impl wl_compositor::Handler for CompositorHandler {
        fn create_surface(&mut self, evlh: &mut EventLoopHandle, _: &Client,
                          _: &wl_compositor::WlCompositor, surface: wl_surface::WlSurface) {
            let hid = self.hid.expect("CompositorHandler was not initialized!");
            evlh.register::<_, CompositorHandler>(&surface, hid);
        }
    }

    impl wl_surface::Handler for CompositorHandler {
        fn attach(&mut self, evqh: &mut EventLoopHandle, _client: &Client, surface: &wl_surface::WlSurface,
                  buffer: Option<&wl_buffer::WlBuffer>, x: i32, y: i32) {
            assert!(buffer.is_none());
            self.got_buffer = true;
        }
    }

    server_declare_handler!(
        CompositorHandler,
        wl_compositor::Handler,
        wl_compositor::WlCompositor
    );
    server_declare_handler!(
        CompositorHandler,
        wl_surface::Handler,
        wl_surface::WlSurface
    );

    pub fn insert_compositor(event_loop: &mut EventLoop) {
        let hid = event_loop.add_handler_with_init(CompositorHandler::new());
        let _ = event_loop.register_global::<wl_compositor::WlCompositor, CompositorHandler>(hid, 1);
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

use self::client_utils::ClientEnv;

#[test]
fn attach_null() {
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

    // create a surface and attach it a null buffer
    //
    {
        let state = client.event_queue.state();
        let env = state.get_handler::<wayc::EnvHandler<ClientEnv>>(client_handler_hid);
        let surface = env.compositor.create_surface();
        surface.attach(None, 0, 0);
    }

    roundtrip(&mut client, &mut server);

    // final assertions
    //
    {
        let state = server.event_loop.state();
        let handler = state.get_handler::<server_utils::CompositorHandler>(0);
        assert!(handler.got_buffer);
    }
}
