#[macro_use]
extern crate wayland_client as wayc;
extern crate wayland_server as ways;

mod helpers;

use helpers::{roundtrip, TestClient, TestServer};

mod server_utils {
    use ways::{EventLoop, StateToken};
    use ways::protocol::{wl_compositor, wl_surface};

    pub struct CompositorState {
        pub got_buffer: bool,
    }

    impl CompositorState {
        fn new() -> CompositorState {
            CompositorState { got_buffer: false }
        }
    }

    fn compositor_impl() -> wl_compositor::Implementation<StateToken<CompositorState>> {
        wl_compositor::Implementation {
            create_surface: |evlh, token, _client, _compositor, surface| {
                evlh.register(&surface, surface_impl(), token.clone());
            },
            create_region: |_, _, _, _, _| {},
        }
    }

    fn surface_impl() -> wl_surface::Implementation<StateToken<CompositorState>> {
        wl_surface::Implementation {
            attach: |evlh, token, _client, _surface, buffer, _, _| {
                assert!(buffer.is_none());
                evlh.state().get_mut(token).got_buffer = true;
            },
            commit: |_, _, _, _| {},
            damage: |_, _, _, _, _, _, _, _| {},
            damage_buffer: |_, _, _, _, _, _, _, _| {},
            destroy: |_, _, _, _| {},
            frame: |_, _, _, _, _| {},
            set_buffer_scale: |_, _, _, _, _| {},
            set_buffer_transform: |_, _, _, _, _| {},
            set_input_region: |_, _, _, _, _| {},
            set_opaque_region: |_, _, _, _, _| {},
        }
    }

    pub fn insert_compositor(event_loop: &mut EventLoop) -> StateToken<CompositorState> {
        let token = event_loop.state().insert(CompositorState::new());
        let _ = event_loop.register_global::<wl_compositor::WlCompositor, _>(
            1,
            |evlh, token, _client, compositor| {
                evlh.register(&compositor, compositor_impl(), token.clone());
            },
            token.clone(),
        );
        token
    }
}

wayland_env!(pub ClientEnv,
    compositor: ::wayc::protocol::wl_compositor::WlCompositor
);

#[test]
fn attach_null() {
    // server setup
    let mut server = TestServer::new();
    let server_comp_token = self::server_utils::insert_compositor(&mut server.event_loop);

    // client setup
    //
    let mut client = TestClient::new(&server.socket_name);
    let client_registry = client.display.get_registry();
    let client_env_token = wayc::EnvHandler::<ClientEnv>::init(&mut client.event_queue, &client_registry);

    // double roundtrip for env init
    //
    roundtrip(&mut client, &mut server);
    roundtrip(&mut client, &mut server);

    // create a surface and attach it a null buffer
    //
    {
        let state = client.event_queue.state();
        let env = state.get(&client_env_token);
        let surface = env.compositor.create_surface();
        surface.attach(None, 0, 0);
    }

    roundtrip(&mut client, &mut server);

    // final assertions
    //
    {
        let state = server.event_loop.state();
        let handler = state.get(&server_comp_token);
        assert!(handler.got_buffer);
    }
}
