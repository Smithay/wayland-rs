mod helpers;

use helpers::{roundtrip, wayc, ways, TestClient, TestServer};

use ways::protocol::wl_output::WlOutput as ServerOutput;
use ways::NewResource;

use wayc::protocol::wl_output::{RequestsTrait, WlOutput};

use std::sync::{Arc, Mutex};

#[test]
fn resource_destructor() {
    let destructor_called = Arc::new(Mutex::new(false));
    let destructor_called_global = destructor_called.clone();

    let mut server = TestServer::new();
    let loop_token = server.event_loop.token();
    server
        .display
        .create_global::<ServerOutput, _>(&loop_token, 3, move |_, newo: NewResource<_>| {
            let destructor_called_resource = destructor_called_global.clone();
            newo.implement(
                |_, _| {},
                Some(move |_, _| {
                    *destructor_called_resource.lock().unwrap() = true;
                }),
            );
        });

    let mut client = TestClient::new(&server.socket_name);
    let manager = wayc::GlobalManager::new(&client.display);

    roundtrip(&mut client, &mut server).unwrap();

    let output = manager
        .instantiate_auto::<WlOutput, _>(|newp| newp.implement(|_, _| {}, ()))
        .unwrap();

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
    let loop_token = server.event_loop.token();
    server
        .display
        .create_global::<ServerOutput, _>(&loop_token, 3, move |_, newo: NewResource<_>| {
            let destructor_called_resource = destructor_called_global.clone();
            newo.implement(
                |_, _| {},
                Some(move |_, _| {
                    *destructor_called_resource.lock().unwrap() = true;
                }),
            );
        });

    let mut client = TestClient::new(&server.socket_name);
    let manager = wayc::GlobalManager::new(&client.display);

    roundtrip(&mut client, &mut server).unwrap();

    manager
        .instantiate_auto::<WlOutput, _>(|newp| newp.implement(|_, _| {}, ()))
        .unwrap();

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
    let loop_token = server.event_loop.token();
    server
        .display
        .create_global::<ServerOutput, _>(&loop_token, 3, move |_, newo: NewResource<_>| {
            let destructor_called_resource = destructor_called_global.clone();
            let output = newo.implement(|_, _| {}, None::<fn(_, _)>);
            let client = output.client().unwrap();
            client.set_user_data(Box::into_raw(Box::new(destructor_called_resource)) as *mut _);
            client.set_destructor(|data| {
                let signal: Box<Arc<Mutex<bool>>> = unsafe { Box::from_raw(data as *mut _) };
                *signal.lock().unwrap() = true;
            });
        });

    let mut client = TestClient::new(&server.socket_name);
    let manager = wayc::GlobalManager::new(&client.display);

    roundtrip(&mut client, &mut server).unwrap();

    manager
        .instantiate_auto::<WlOutput, _>(|newp| newp.implement(|_, _| {}, ()))
        .unwrap();

    roundtrip(&mut client, &mut server).unwrap();

    ::std::mem::drop(manager);
    ::std::mem::drop(client);

    server.answer();

    assert!(*destructor_called.lock().unwrap());
}
