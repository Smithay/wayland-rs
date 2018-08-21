mod helpers;

use helpers::{roundtrip, wayc, ways, TestClient, TestServer};

use ways::protocol::wl_output;
use ways::{Implementation, NewResource, Resource};

use wayc::protocol::wl_output::WlOutput as ClientOutput;

use std::sync::{Arc, Mutex};

struct Impl1;

impl Implementation<Resource<wl_output::WlOutput>, wl_output::Request> for Impl1 {
    fn receive(&mut self, _: wl_output::Request, _: Resource<wl_output::WlOutput>) {}
}

struct Impl2;

impl Implementation<Resource<wl_output::WlOutput>, wl_output::Request> for Impl2 {
    fn receive(&mut self, _: wl_output::Request, _: Resource<wl_output::WlOutput>) {}
}

#[test]
fn resource_equals() {
    let mut server = TestServer::new();
    let loop_token = server.event_loop.token();

    let outputs = Arc::new(Mutex::new(Vec::new()));
    let outputs2 = outputs.clone();

    server
        .display
        .create_global::<wl_output::WlOutput, _>(&loop_token, 1, move |_, newo: NewResource<_>| {
            outputs2
                .lock()
                .unwrap()
                .push(newo.implement(|_, _| {}, None::<fn(_, _)>, ()));
        });

    let mut client = TestClient::new(&server.socket_name);
    let manager = wayc::GlobalManager::new(&client.display);

    roundtrip(&mut client, &mut server).unwrap();

    // create two outputs
    manager
        .instantiate_auto::<ClientOutput, _>(|newp| newp.implement(|_, _| {}, ()))
        .unwrap();
    manager
        .instantiate_auto::<ClientOutput, _>(|newp| newp.implement(|_, _| {}, ()))
        .unwrap();

    roundtrip(&mut client, &mut server).unwrap();

    let outputs_lock = outputs.lock().unwrap();
    assert!(outputs_lock.len() == 2);
    assert!(outputs_lock[0] != outputs_lock[1]);

    let cloned = outputs_lock[0].clone();
    assert!(outputs_lock[0] == cloned);

    assert!(outputs_lock[0].same_client_as(&outputs_lock[1]));
}

#[test]
fn resource_user_data() {
    let mut server = TestServer::new();
    let loop_token = server.event_loop.token();

    let outputs = Arc::new(Mutex::new(Vec::new()));
    let outputs2 = outputs.clone();

    server
        .display
        .create_global::<wl_output::WlOutput, _>(&loop_token, 1, move |_, newo: NewResource<_>| {
            let mut guard = outputs2.lock().unwrap();
            let output = newo.implement(|_, _| {}, None::<fn(_, _)>, 1000 + guard.len());
            guard.push(output);
        });

    let mut client = TestClient::new(&server.socket_name);
    let manager = wayc::GlobalManager::new(&client.display);

    roundtrip(&mut client, &mut server).unwrap();

    // create two outputs
    manager
        .instantiate_auto::<ClientOutput, _>(|newp| newp.implement(|_, _| {}, ()))
        .unwrap();
    manager
        .instantiate_auto::<ClientOutput, _>(|newp| newp.implement(|_, _| {}, ()))
        .unwrap();

    roundtrip(&mut client, &mut server).unwrap();

    let outputs_lock = outputs.lock().unwrap();
    assert!(outputs_lock[0].user_data::<usize>() == Some(&1000));
    assert!(outputs_lock[1].user_data::<usize>() == Some(&1001));
    let cloned = outputs_lock[0].clone();
    assert!(cloned.user_data::<usize>() == Some(&1000));
}

#[test]
fn resource_user_data_wrong_thread() {
    let mut server = TestServer::new();
    let loop_token = server.event_loop.token();
    let loop_token2 = loop_token.clone();

    let outputs = Arc::new(Mutex::new(None));
    let outputs2 = outputs.clone();

    server
        .display
        .create_global::<wl_output::WlOutput, _>(&loop_token, 1, move |_, newo: NewResource<_>| {
            let mut guard = outputs2.lock().unwrap();
            let output = newo.implement_nonsend(|_, _| {}, None::<fn(_, _)>, 0xDEADBEEFusize, &loop_token2);
            *guard = Some(output);
        });

    let mut client = TestClient::new(&server.socket_name);
    let manager = wayc::GlobalManager::new(&client.display);

    roundtrip(&mut client, &mut server).unwrap();

    manager
        .instantiate_auto::<ClientOutput, _>(|newp| newp.implement(|_, _| {}, ()))
        .unwrap();

    roundtrip(&mut client, &mut server).unwrap();

    let output = outputs.lock().unwrap().take().unwrap();

    // we can access on the right thread
    assert!(output.user_data::<usize>().is_some());

    // but not in a new one
    ::std::thread::spawn(move || {
        assert!(output.user_data::<usize>().is_none());
    }).join()
        .unwrap();
}

#[test]
fn resource_is_implemented() {
    let mut server = TestServer::new();
    let loop_token = server.event_loop.token();

    let outputs = Arc::new(Mutex::new(Vec::new()));
    let outputs2 = outputs.clone();

    server
        .display
        .create_global::<wl_output::WlOutput, _>(&loop_token, 1, move |_, newo: NewResource<_>| {
            let mut guard = outputs2.lock().unwrap();
            let output = if guard.len() == 0 {
                newo.implement(Impl1, None::<fn(_, _)>, ())
            } else {
                newo.implement(Impl2, None::<fn(_, _)>, ())
            };
            guard.push(output);
        });

    let mut client = TestClient::new(&server.socket_name);
    let manager = wayc::GlobalManager::new(&client.display);

    roundtrip(&mut client, &mut server).unwrap();

    // create two outputs
    manager
        .instantiate_auto::<ClientOutput, _>(|newp| newp.implement(|_, _| {}, ()))
        .unwrap();
    manager
        .instantiate_auto::<ClientOutput, _>(|newp| newp.implement(|_, _| {}, ()))
        .unwrap();

    roundtrip(&mut client, &mut server).unwrap();

    let outputs_lock = outputs.lock().unwrap();
    assert!(outputs_lock[0].is_implemented_with::<Impl1>());
    assert!(!outputs_lock[0].is_implemented_with::<Impl2>());
    assert!(outputs_lock[1].is_implemented_with::<Impl2>());
    assert!(!outputs_lock[1].is_implemented_with::<Impl1>());
    let cloned = outputs_lock[0].clone();
    assert!(cloned.is_implemented_with::<Impl1>());
    assert!(!cloned.is_implemented_with::<Impl2>());
}

#[test]
fn dead_resources() {
    use self::wayc::protocol::wl_output::RequestsTrait;
    let mut server = TestServer::new();
    let loop_token = server.event_loop.token();

    let outputs = Arc::new(Mutex::new(Vec::new()));
    let outputs2 = outputs.clone();

    server
        .display
        .create_global::<wl_output::WlOutput, _>(&loop_token, 3, move |_, newo: NewResource<_>| {
            outputs2
                .lock()
                .unwrap()
                .push(newo.implement(|_, _| {}, None::<fn(_, _)>, ()));
        });

    let mut client = TestClient::new(&server.socket_name);
    let manager = wayc::GlobalManager::new(&client.display);

    roundtrip(&mut client, &mut server).unwrap();

    let client_output1 = manager
        .instantiate_auto::<ClientOutput, _>(|newp| newp.implement(|_, _| {}, ()))
        .unwrap();
    manager
        .instantiate_auto::<ClientOutput, _>(|newp| newp.implement(|_, _| {}, ()))
        .unwrap();

    roundtrip(&mut client, &mut server).unwrap();

    let cloned = {
        let outputs_lock = outputs.lock().unwrap();
        assert!(outputs_lock[0].is_alive());
        assert!(outputs_lock[1].is_alive());
        outputs_lock[0].clone()
    };

    client_output1.release();

    roundtrip(&mut client, &mut server).unwrap();

    {
        let outputs_lock = outputs.lock().unwrap();
        assert!(!outputs_lock[0].is_alive());
        assert!(outputs_lock[1].is_alive());
        assert!(!cloned.is_alive());
    }
}
