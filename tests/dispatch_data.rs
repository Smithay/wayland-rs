mod helpers;

use helpers::{roundtrip, wayc, ways, TestClient, TestServer};

use wayc::protocol::wl_compositor;
use ways::protocol::wl_compositor::WlCompositor as ServerCompositor;

use std::time::Duration;

#[test]
fn client_dispatch_data() {
    let mut server = TestServer::new();

    let mut client = TestClient::new(&server.socket_name);

    // do a manual roundtrip
    let mut done = false;
    client.display_proxy.sync().quick_assign(move |_, _, mut data| {
        let done = data.get::<bool>().unwrap();
        *done = true;
    });
    client.display.flush().unwrap();
    server.answer();
    client
        .event_queue
        .dispatch(&mut done, |_, _, _| unreachable!())
        .unwrap();
    assert!(done);
}

#[test]
fn server_dispatch_data_global() {
    let mut server = TestServer::new();
    server
        .display
        .create_global::<ServerCompositor, _>(1, |_, _, mut data| {
            let done = data.get::<bool>().unwrap();
            *done = true;
        });

    let mut client = TestClient::new(&server.socket_name);
    let manager = wayc::GlobalManager::new(&client.display_proxy);
    roundtrip(&mut client, &mut server).unwrap();

    manager
        .instantiate_exact::<wl_compositor::WlCompositor>(1)
        .unwrap();
    client.display.flush().unwrap();

    let mut done = false;
    server
        .display
        .dispatch(Duration::from_millis(10), &mut done)
        .unwrap();
    assert!(done);
}

#[test]
fn server_dispatch_data_client_destructor() {
    let mut server = TestServer::new();
    server
        .display
        .create_global::<ServerCompositor, _>(1, move |compositor, _, _| {
            compositor
                .as_ref()
                .client()
                .unwrap()
                .add_destructor(ways::Filter::new(|_, _, mut data| {
                    let done = data.get::<bool>().unwrap();
                    *done = true;
                }));
        });

    let mut client = TestClient::new(&server.socket_name);
    let manager = wayc::GlobalManager::new(&client.display_proxy);
    roundtrip(&mut client, &mut server).unwrap();

    manager
        .instantiate_exact::<wl_compositor::WlCompositor>(1)
        .unwrap();
    roundtrip(&mut client, &mut server).unwrap();

    // the client detructor should be in place and ready

    // disconnect the client
    std::mem::drop(manager);
    std::mem::drop(client);

    // process the destructor
    let mut done = false;
    // system_lib / rust_impl do not process destructors at the same time
    server
        .display
        .dispatch(Duration::from_millis(10), &mut done)
        .unwrap();
    server.display.flush_clients(&mut done);
    assert!(done);
}

#[test]
fn server_dispatch_data_resource() {
    let mut server = TestServer::new();
    server
        .display
        .create_global::<ServerCompositor, _>(1, move |compositor, _, _| {
            compositor.quick_assign(|_, _, mut data| {
                let done = data.get::<bool>().unwrap();
                *done = true;
            });
        });

    let mut client = TestClient::new(&server.socket_name);
    let manager = wayc::GlobalManager::new(&client.display_proxy);
    roundtrip(&mut client, &mut server).unwrap();

    let compositor = manager
        .instantiate_exact::<wl_compositor::WlCompositor>(1)
        .unwrap();
    roundtrip(&mut client, &mut server).unwrap();

    // the resource filter should be in place and ready
    compositor.create_surface();
    client.display.flush().unwrap();

    // process the event
    let mut done = false;
    server
        .display
        .dispatch(Duration::from_millis(10), &mut done)
        .unwrap();
    assert!(done);
}

#[test]
fn server_dispatch_data_resource_destructor() {
    let mut server = TestServer::new();
    server
        .display
        .create_global::<ServerCompositor, _>(1, move |compositor, _, _| {
            compositor.assign_destructor(ways::Filter::new(|_: ways::Resource<_>, _, mut data| {
                let done = data.get::<bool>().unwrap();
                *done = true;
            }));
        });

    let mut client = TestClient::new(&server.socket_name);
    let manager = wayc::GlobalManager::new(&client.display_proxy);
    roundtrip(&mut client, &mut server).unwrap();

    manager
        .instantiate_exact::<wl_compositor::WlCompositor>(1)
        .unwrap();
    roundtrip(&mut client, &mut server).unwrap();

    // the resource detructor should be in place and ready

    // disconnect the client
    std::mem::drop(manager);
    std::mem::drop(client);

    // process the destructor
    let mut done = false;
    // system_lib / rust_impl do not process destructors at the same time
    server
        .display
        .dispatch(Duration::from_millis(10), &mut done)
        .unwrap();
    server.display.flush_clients(&mut done);
    assert!(done);
}
