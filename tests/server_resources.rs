mod helpers;

use helpers::{roundtrip, wayc, ways, TestClient, TestServer};

use ways::protocol::wl_output;

use wayc::protocol::wl_output::WlOutput as ClientOutput;

use std::sync::{Arc, Mutex};

#[test]
fn resource_equals() {
    let mut server = TestServer::new();

    let outputs = Arc::new(Mutex::new(Vec::new()));
    let outputs2 = outputs.clone();

    server
        .display
        .create_global::<wl_output::WlOutput, _>(1, move |newo, _, _| {
            outputs2.lock().unwrap().push(newo);
        });

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

    server
        .display
        .create_global::<wl_output::WlOutput, _>(1, move |output, _, _| {
            let mut guard = outputs2.lock().unwrap();
            output.as_ref().user_data().set(|| 1000 + guard.len());
            guard.push(output);
        });

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

    server
        .display
        .create_global::<wl_output::WlOutput, _>(3, move |newo, _, _| {
            newo.quick_assign(|_, _, _| {});
            outputs2.lock().unwrap().push(newo);
        });

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
