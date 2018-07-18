use std::sync::{Arc, Mutex};

mod helpers;

use helpers::{roundtrip, wayc, ways, TestClient, TestServer};

use wayc::protocol::wl_compositor::RequestsTrait as CompositorRequests;
use wayc::protocol::wl_display::RequestsTrait as DisplayRequests;
use wayc::protocol::wl_surface::RequestsTrait as SurfaceRequests;

fn insert_compositor(server: &mut TestServer) -> Arc<Mutex<bool>> {
    use ways::protocol::{wl_compositor, wl_surface};
    use ways::NewResource;

    let seen_surface = Arc::new(Mutex::new(false));
    let seen_surface2 = seen_surface.clone();

    let loop_token = server.event_loop.token();
    server.display.create_global::<wl_compositor::WlCompositor, _>(
        &loop_token,
        1,
        move |version, compositor: NewResource<_>| {
            assert!(version == 1);
            let compositor_seen_surface = seen_surface.clone();
            compositor.implement(
                move |event, _| {
                    if let wl_compositor::Request::CreateSurface { id: surface } = event {
                        let my_seen_surface = compositor_seen_surface.clone();
                        surface.implement(
                            move |event, _| {
                                if let wl_surface::Request::Attach { buffer, x, y } = event {
                                    assert!(buffer.is_none());
                                    assert!(x == 0);
                                    assert!(y == 0);
                                    *(my_seen_surface.lock().unwrap()) = true;
                                } else {
                                    panic!("Unexpected event on surface!");
                                }
                            },
                            None::<fn(_, _)>,
                        );
                    } else {
                        panic!("Unexpected event on compositor!");
                    }
                },
                None::<fn(_, _)>,
            );
        },
    );

    seen_surface2
}

#[test]
fn attach_null() {
    // Server setup
    //
    let mut server = TestServer::new();
    let seen_surface = insert_compositor(&mut server);

    // Client setup
    //
    let mut client = TestClient::new(&server.socket_name);
    let manager = wayc::GlobalManager::new(&client.display);

    // Initial sync
    roundtrip(&mut client, &mut server);

    let compositor = manager
        .instantiate_exact::<wayc::protocol::wl_compositor::WlCompositor, _>(1, |comp| {
            comp.implement(|_, _| {})
        })
        .unwrap();
    let surface = compositor
        .create_surface(|surface| surface.implement(|_, _| {}))
        .unwrap();
    surface.attach(None, 0, 0);

    roundtrip(&mut client, &mut server);

    assert!(*seen_surface.lock().unwrap());
}
