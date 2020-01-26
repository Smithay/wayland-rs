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
    server.display.create_global::<ServerOutput, _>(
        3,
        ways::Filter::new(move |(newo, _): (ways::Main<ServerOutput>, _), _, _| {
            let destructor_called_resource = destructor_called_global.clone();
            newo.quick_assign(|_, _, _| {});
            newo.assign_destructor(ways::Filter::new(move |_: ways::Resource<_>, _, _| {
                *destructor_called_resource.lock().unwrap() = true;
            }));
        }),
    );

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
    server.display.create_global::<ServerOutput, _>(
        3,
        ways::Filter::new(move |(newo, _): (ways::Main<ServerOutput>, _), _, _| {
            let destructor_called_resource = destructor_called_global.clone();
            newo.assign_destructor(ways::Filter::new(move |_: ways::Resource<_>, _, _| {
                *destructor_called_resource.lock().unwrap() = true;
            }));
        }),
    );

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
    server.display.create_global::<ServerOutput, _>(
        3,
        ways::Filter::new(move |(output, _): (ways::Main<ServerOutput>, _), _, _| {
            let destructor_called_resource = destructor_called_global.clone();
            let client = output.as_ref().client().unwrap();
            client.add_destructor(ways::Filter::new(move |_, _, _| {
                *destructor_called_resource.lock().unwrap() = true;
            }));
        }),
    );

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
