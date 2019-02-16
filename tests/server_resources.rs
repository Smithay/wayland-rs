mod helpers;

use helpers::{roundtrip, wayc, ways, TestClient, TestServer};

use ways::protocol::wl_output;

use wayc::protocol::wl_output::WlOutput as ClientOutput;

use std::os::unix::io::IntoRawFd;
use std::sync::{Arc, Mutex};

#[test]
fn resource_equals() {
    let mut server = TestServer::new();

    let outputs = Arc::new(Mutex::new(Vec::new()));
    let outputs2 = outputs.clone();

    server
        .display
        .create_global::<wl_output::WlOutput, _>(1, move |newo, _| {
            outputs2.lock().unwrap().push(newo.implement_dummy());
        });

    let mut client = TestClient::new(&server.socket_name);
    let manager = wayc::GlobalManager::new(&client.display);

    roundtrip(&mut client, &mut server).unwrap();

    // create two outputs
    manager
        .instantiate_exact::<ClientOutput, _>(1, |newp| newp.implement_dummy())
        .unwrap();
    manager
        .instantiate_exact::<ClientOutput, _>(1, |newp| newp.implement_dummy())
        .unwrap();

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
        .create_global::<wl_output::WlOutput, _>(1, move |newo, _| {
            let mut guard = outputs2.lock().unwrap();
            let output = newo.implement_closure(|_, _| {}, None::<fn(_)>, 1000 + guard.len());
            guard.push(output);
        });

    let mut client = TestClient::new(&server.socket_name);
    let manager = wayc::GlobalManager::new(&client.display);

    roundtrip(&mut client, &mut server).unwrap();

    // create two outputs
    manager
        .instantiate_exact::<ClientOutput, _>(1, |newp| newp.implement_dummy())
        .unwrap();
    manager
        .instantiate_exact::<ClientOutput, _>(1, |newp| newp.implement_dummy())
        .unwrap();

    roundtrip(&mut client, &mut server).unwrap();

    let outputs_lock = outputs.lock().unwrap();
    assert!(outputs_lock[0].as_ref().user_data::<usize>() == Some(&1000));
    assert!(outputs_lock[1].as_ref().user_data::<usize>() == Some(&1001));
    let cloned = outputs_lock[0].clone();
    assert!(cloned.as_ref().user_data::<usize>() == Some(&1000));
}

#[cfg(not(feature = "server_native"))]
#[test]
fn resource_user_data_wrong_thread() {
    let mut server = TestServer::new();

    let outputs = Arc::new(Mutex::new(None));
    let outputs2 = outputs.clone();

    server
        .display
        .create_global::<wl_output::WlOutput, _>(1, move |newo, _| {
            let mut guard = outputs2.lock().unwrap();
            let output = newo.implement_closure(|_, _| {}, None::<fn(_)>, 0xDEADBEEFusize);
            *guard = Some(output);
        });

    let mut client = TestClient::new(&server.socket_name);
    let manager = wayc::GlobalManager::new(&client.display);

    roundtrip(&mut client, &mut server).unwrap();

    manager
        .instantiate_exact::<ClientOutput, _>(1, |newp| newp.implement_dummy())
        .unwrap();

    roundtrip(&mut client, &mut server).unwrap();

    let output = outputs.lock().unwrap().take().unwrap();

    // we can access on the right thread
    assert!(output.as_ref().user_data::<usize>().is_some());

    // but not in a new one
    ::std::thread::spawn(move || {
        assert!(output.as_ref().user_data::<usize>().is_none());
    })
    .join()
    .unwrap();
}

#[cfg(not(feature = "server_native"))]
#[test]
fn resource_implement_wrong_thread() {
    let server = TestServer::new();

    let (s1, _) = ::std::os::unix::net::UnixStream::pair().unwrap();
    let my_client = unsafe { server.display.create_client(s1.into_raw_fd()) };

    let ret = ::std::thread::spawn(move || {
        let newp = my_client.create_resource::<wl_output::WlOutput>(1).unwrap();
        newp.implement_closure(|_, _| {}, None::<fn(_)>, ()); // should panic
    })
    .join();

    // child thread should have panicked
    assert!(ret.is_err());
}

#[test]
fn dead_resources() {
    let mut server = TestServer::new();

    let outputs = Arc::new(Mutex::new(Vec::new()));
    let outputs2 = outputs.clone();

    server
        .display
        .create_global::<wl_output::WlOutput, _>(3, move |newo, _| {
            outputs2.lock().unwrap().push(newo.implement_dummy());
        });

    let mut client = TestClient::new(&server.socket_name);
    let manager = wayc::GlobalManager::new(&client.display);

    roundtrip(&mut client, &mut server).unwrap();

    let client_output1 = manager
        .instantiate_exact::<ClientOutput, _>(3, |newp| newp.implement_dummy())
        .unwrap();
    manager
        .instantiate_exact::<ClientOutput, _>(3, |newp| newp.implement_dummy())
        .unwrap();

    roundtrip(&mut client, &mut server).unwrap();

    let cloned = {
        let outputs_lock = outputs.lock().unwrap();
        assert!(outputs_lock[0].as_ref().is_alive());
        assert!(outputs_lock[1].as_ref().is_alive());
        outputs_lock[0].as_ref().clone()
    };

    client_output1.release();

    roundtrip(&mut client, &mut server).unwrap();

    {
        let outputs_lock = outputs.lock().unwrap();
        assert!(!outputs_lock[0].as_ref().is_alive());
        assert!(outputs_lock[1].as_ref().is_alive());
        assert!(!cloned.is_alive());
    }
}
