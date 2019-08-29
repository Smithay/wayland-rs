mod helpers;

use helpers::{roundtrip, wayc, ways, TestClient, TestServer};

use ways::protocol::wl_output::WlOutput as ServerOutput;

use wayc::protocol::wl_output::WlOutput;

use std::sync::{Arc, Mutex};

#[test]
fn resource_destructor_request() {
    let destructor_called = Arc::new(Mutex::new(false));
    let destructor_called_global = destructor_called.clone();

    let mut server = TestServer::new();
    server
        .display
        .create_global::<ServerOutput, _>(3, move |newo, _| {
            let destructor_called_resource = destructor_called_global.clone();
            newo.assign_mono(|_, _| {});
            newo.assign_destructor(ways::Filter::new(move |_: ways::Resource<_>, _| {
                *destructor_called_resource.lock().unwrap() = true;
            }));
        });

    let mut client = TestClient::new(&server.socket_name);
    let manager = wayc::GlobalManager::new(&client.display_proxy);

    roundtrip(&mut client, &mut server).unwrap();

    let output = manager.instantiate_exact::<WlOutput>(3).unwrap();

    roundtrip(&mut client, &mut server).unwrap();

    output.release();

    roundtrip(&mut client, &mut server).unwrap();

    assert!(*destructor_called.lock().unwrap());
}

#[test]
fn resource_destructor_cleanup() {
    let destructor_called = Arc::new(Mutex::new(false));
    let destructor_called_global = destructor_called.clone();

    let mut server = TestServer::new();
    server
        .display
        .create_global::<ServerOutput, _>(3, move |newo, _| {
            let destructor_called_resource = destructor_called_global.clone();
            newo.assign_destructor(ways::Filter::new(move |_: ways::Resource<_>, _| {
                *destructor_called_resource.lock().unwrap() = true;
            }));
        });

    let mut client = TestClient::new(&server.socket_name);
    let manager = wayc::GlobalManager::new(&client.display_proxy);

    roundtrip(&mut client, &mut server).unwrap();

    manager.instantiate_exact::<WlOutput>(3).unwrap();

    roundtrip(&mut client, &mut server).unwrap();

    ::std::mem::drop(manager);
    ::std::mem::drop(client);

    server.answer();

    assert!(*destructor_called.lock().unwrap());
}

#[test]
fn client_destructor_cleanup() {
    let destructor_called = Arc::new(Mutex::new(false));
    let destructor_called_global = destructor_called.clone();

    let mut server = TestServer::new();
    server
        .display
        .create_global::<ServerOutput, _>(3, move |output, _| {
            let destructor_called_resource = destructor_called_global.clone();
            let client = output.client().unwrap();
            client.add_destructor(ways::Filter::new(move |_, _| {
                *destructor_called_resource.lock().unwrap() = true;
            }));
        });

    let mut client = TestClient::new(&server.socket_name);
    let manager = wayc::GlobalManager::new(&client.display_proxy);

    roundtrip(&mut client, &mut server).unwrap();

    manager.instantiate_exact::<WlOutput>(3).unwrap();

    roundtrip(&mut client, &mut server).unwrap();

    ::std::mem::drop(manager);
    ::std::mem::drop(client);

    server.answer();

    assert!(*destructor_called.lock().unwrap());
}
