mod helpers;

use helpers::{roundtrip, wayc, ways, TestClient, TestServer};

use ways::protocol::{wl_compositor, wl_output};

use wayc::protocol::wl_output::WlOutput as ClientOutput;

use std::sync::{Arc, Mutex};

#[test]
fn resource_equals() {
    let mut server = TestServer::new();

    let outputs = Arc::new(Mutex::new(Vec::new()));
    let outputs2 = outputs.clone();

    server.display.create_global::<wl_output::WlOutput, _>(
        1,
        ways::Filter::new(move |(newo, _): (ways::Main<wl_output::WlOutput>, u32), _, _| {
            outputs2.lock().unwrap().push(newo);
        }),
    );

    let mut client = TestClient::new(&server.socket_name);
    let manager = wayc::GlobalManager::new(&client.display_proxy);

    roundtrip(&mut client, &mut server).unwrap();

    // create two outputs
    manager.instantiate_exact::<ClientOutput>(1).unwrap();
    manager.instantiate_exact::<ClientOutput>(1).unwrap();

    roundtrip(&mut client, &mut server).unwrap();

    let outputs_lock = outputs.lock().unwrap();
    assert!(outputs_lock.len() == 2);
    assert!(outputs_lock[0] != outputs_lock[1]);

    let cloned = outputs_lock[0].clone();
    assert!(outputs_lock[0] == cloned);

    assert!(outputs_lock[0].as_ref().same_client_as(outputs_lock[1].as_ref()));
}

#[test]
fn resource_user_data() {
    let mut server = TestServer::new();

    let outputs = Arc::new(Mutex::new(Vec::new()));
    let outputs2 = outputs.clone();

    server.display.create_global::<wl_output::WlOutput, _>(
        1,
        ways::Filter::new(move |(output, _): (ways::Main<wl_output::WlOutput>, u32), _, _| {
            let mut guard = outputs2.lock().unwrap();
            output.as_ref().user_data().set(|| 1000 + guard.len());
            guard.push(output);
        }),
    );

    let mut client = TestClient::new(&server.socket_name);
    let manager = wayc::GlobalManager::new(&client.display_proxy);

    roundtrip(&mut client, &mut server).unwrap();

    // create two outputs
    manager.instantiate_exact::<ClientOutput>(1).unwrap();
    manager.instantiate_exact::<ClientOutput>(1).unwrap();

    roundtrip(&mut client, &mut server).unwrap();

    let outputs_lock = outputs.lock().unwrap();
    assert!(outputs_lock[0].as_ref().user_data().get::<usize>() == Some(&1000));
    assert!(outputs_lock[1].as_ref().user_data().get::<usize>() == Some(&1001));
    let cloned = outputs_lock[0].clone();
    assert!(cloned.as_ref().user_data().get::<usize>() == Some(&1000));
}

#[test]
fn dead_resources() {
    let mut server = TestServer::new();

    let outputs = Arc::new(Mutex::new(Vec::new()));
    let outputs2 = outputs.clone();

    server.display.create_global::<wl_output::WlOutput, _>(
        3,
        ways::Filter::new(move |(newo, _): (ways::Main<wl_output::WlOutput>, u32), _, _| {
            newo.quick_assign(|_, _, _| {});
            outputs2.lock().unwrap().push(newo);
        }),
    );

    let mut client = TestClient::new(&server.socket_name);
    let manager = wayc::GlobalManager::new(&client.display_proxy);

    roundtrip(&mut client, &mut server).unwrap();

    let client_output1 = manager.instantiate_exact::<ClientOutput>(3).unwrap();
    manager.instantiate_exact::<ClientOutput>(3).unwrap();

    roundtrip(&mut client, &mut server).unwrap();

    let cloned = {
        let outputs_lock = outputs.lock().unwrap();
        assert!(outputs_lock[0].as_ref().is_alive());
        assert!(outputs_lock[1].as_ref().is_alive());
        outputs_lock[0].clone()
    };

    client_output1.release();

    roundtrip(&mut client, &mut server).unwrap();

    {
        let outputs_lock = outputs.lock().unwrap();
        assert!(!outputs_lock[0].as_ref().is_alive());
        assert!(outputs_lock[1].as_ref().is_alive());
        assert!(!cloned.as_ref().is_alive());
    }
}

#[test]
fn get_resource() {
    let mut server = TestServer::new();
    let clients = Arc::new(Mutex::new(Vec::new()));

    server.display.create_global::<wl_output::WlOutput, _>(1, {
        let clients = clients.clone();
        ways::Filter::new(move |(output, _): (ways::Main<wl_output::WlOutput>, u32), _, _| {
            // retrieve and store the client
            let client = output.as_ref().client().unwrap();
            clients.lock().unwrap().push(client);
        })
    });

    let mut client = TestClient::new(&server.socket_name);
    let manager = wayc::GlobalManager::new(&client.display_proxy);

    roundtrip(&mut client, &mut server).unwrap();

    // Instantiate a global
    manager.instantiate_exact::<ClientOutput>(1).unwrap();

    roundtrip(&mut client, &mut server).unwrap();

    // try to retrieve the resource
    // its id should be 3 (1 is wl_display and 2 is wl_registry)
    let clients = clients.lock().unwrap();
    // wrong interface fails
    assert!(clients[0].get_resource::<wl_compositor::WlCompositor>(3).is_none());
    // wrong id fails
    assert!(clients[0].get_resource::<wl_output::WlOutput>(4).is_none());
    // but this suceeds
    assert!(clients[0].get_resource::<wl_output::WlOutput>(3).is_some());
}
