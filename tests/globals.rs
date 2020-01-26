mod helpers;

use helpers::{roundtrip, wayc, ways, TestClient, TestServer};

use ways::protocol::wl_compositor::WlCompositor as ServerCompositor;
use ways::protocol::wl_output::WlOutput as ServerOutput;
use ways::protocol::wl_shell::WlShell as ServerShell;

use std::sync::{Arc, Mutex};

#[test]
fn simple_global() {
    let mut server = TestServer::new();
    server
        .display
        .create_global::<ServerCompositor, _>(1, ways::Filter::new(|_: (_, _), _, _| {}));

    let mut client = TestClient::new(&server.socket_name);
    let manager = wayc::GlobalManager::new(&client.display_proxy);

    roundtrip(&mut client, &mut server).unwrap();
    let globals = manager.list();
    assert!(globals.len() == 1);
    assert_eq!(globals[0], (1, "wl_compositor".into(), 1));
}

#[test]
fn multi_versions() {
    let mut server = TestServer::new();
    server
        .display
        .create_global::<ServerCompositor, _>(4, ways::Filter::new(|_: (_, _), _, _| {}));
    server
        .display
        .create_global::<ServerCompositor, _>(3, ways::Filter::new(|_: (_, _), _, _| {}));
    server
        .display
        .create_global::<ServerCompositor, _>(2, ways::Filter::new(|_: (_, _), _, _| {}));
    server
        .display
        .create_global::<ServerCompositor, _>(1, ways::Filter::new(|_: (_, _), _, _| {}));

    let mut client = TestClient::new(&server.socket_name);
    let manager = wayc::GlobalManager::new(&client.display_proxy);

    roundtrip(&mut client, &mut server).unwrap();
    let globals = manager.list();
    assert!(globals.len() == 4);
    let mut seen = [false; 4];
    for &(_, ref interface, version) in &globals {
        assert!(interface == "wl_compositor");
        seen[version as usize - 1] = true;
    }
    assert_eq!(seen, [true, true, true, true]);
}

#[test]
fn dynamic_global() {
    let mut server = TestServer::new();
    server
        .display
        .create_global::<ServerCompositor, _>(1, ways::Filter::new(|_: (_, _), _, _| {}));

    let mut client = TestClient::new(&server.socket_name);
    let manager = wayc::GlobalManager::new(&client.display_proxy);

    roundtrip(&mut client, &mut server).unwrap();
    assert!(manager.list().len() == 1);

    server
        .display
        .create_global::<ServerShell, _>(1, ways::Filter::new(|_: (_, _), _, _| {}));

    roundtrip(&mut client, &mut server).unwrap();
    assert!(manager.list().len() == 2);

    let output = server
        .display
        .create_global::<ServerOutput, _>(1, ways::Filter::new(|_: (_, _), _, _| {}));

    roundtrip(&mut client, &mut server).unwrap();
    assert!(manager.list().len() == 3);

    output.destroy();

    roundtrip(&mut client, &mut server).unwrap();
    assert!(manager.list().len() == 2);
}

#[test]
fn global_manager_cb() {
    use wayc::GlobalEvent;

    let mut server = TestServer::new();
    server
        .display
        .create_global::<ServerCompositor, _>(1, ways::Filter::new(|_: (_, _), _, _| {}));

    let counter = Arc::new(Mutex::new(0));
    let counter2 = counter.clone();

    let mut client = TestClient::new(&server.socket_name);
    let manager = wayc::GlobalManager::new_with_cb(&client.display_proxy, move |event, _, _| match event {
        GlobalEvent::New { .. } => *(counter2.lock().unwrap()) += 1,
        GlobalEvent::Removed { .. } => *(counter2.lock().unwrap()) -= 1,
    });

    roundtrip(&mut client, &mut server).unwrap();

    server
        .display
        .create_global::<ServerCompositor, _>(1, ways::Filter::new(|_: (_, _), _, _| {}));
    server
        .display
        .create_global::<ServerCompositor, _>(1, ways::Filter::new(|_: (_, _), _, _| {}));
    let comp = server
        .display
        .create_global::<ServerCompositor, _>(1, ways::Filter::new(|_: (_, _), _, _| {}));

    roundtrip(&mut client, &mut server).unwrap();

    assert!(manager.list().len() == 4);
    assert!(*counter.lock().unwrap() == 4);

    comp.destroy();

    roundtrip(&mut client, &mut server).unwrap();

    assert!(manager.list().len() == 3);
    assert!(*counter.lock().unwrap() == 3);
}

#[test]
fn range_instantiate() {
    use wayc::protocol::wl_compositor::WlCompositor;
    use wayc::protocol::wl_output::WlOutput;
    use wayc::protocol::wl_shell::WlShell;
    use wayc::GlobalError;

    let mut server = TestServer::new();
    server
        .display
        .create_global::<ServerCompositor, _>(4, ways::Filter::new(|_: (_, _), _, _| {}));
    server
        .display
        .create_global::<ServerShell, _>(1, ways::Filter::new(|_: (_, _), _, _| {}));

    let mut client = TestClient::new(&server.socket_name);
    let manager = wayc::GlobalManager::new(&client.display_proxy);

    roundtrip(&mut client, &mut server).unwrap();

    let compositor = manager.instantiate_range::<WlCompositor>(1, 4).unwrap();
    assert!(compositor.as_ref().version() == 4);
    let shell = manager.instantiate_range::<WlShell>(1, 3).unwrap();
    assert!(shell.as_ref().version() == 1);

    assert!(manager.instantiate_exact::<WlCompositor>(5) == Err(GlobalError::VersionTooLow(4)));
    assert!(manager.instantiate_exact::<WlOutput>(5) == Err(GlobalError::Missing));
    assert!(manager.instantiate_range::<WlOutput>(1, 3) == Err(GlobalError::Missing));
}

#[test]
#[should_panic]
fn wrong_version_create_global() {
    let mut server = TestServer::new();
    server
        .display
        .create_global::<ServerCompositor, _>(42, ways::Filter::new(|_: (_, _), _, _| {}));
}

#[test]
#[cfg_attr(feature = "server_native", ignore)]
fn wrong_global() {
    use wayc::protocol::wl_output::WlOutput;

    let mut server = TestServer::new();
    server
        .display
        .create_global::<ServerCompositor, _>(1, ways::Filter::new(|_: (_, _), _, _| {}));

    let mut client = TestClient::new(&server.socket_name);
    let registry = client.display_proxy.get_registry();

    // instantiate a wrong global, this should kill the client
    // but currently does not fail on native_lib

    registry.bind::<WlOutput>(1, 1);

    assert!(roundtrip(&mut client, &mut server).is_err());
}

#[test]
fn wrong_global_version() {
    use wayc::protocol::wl_compositor::WlCompositor;

    let mut server = TestServer::new();
    server
        .display
        .create_global::<ServerCompositor, _>(1, ways::Filter::new(|_: (_, _), _, _| {}));

    let mut client = TestClient::new(&server.socket_name);
    let registry = client.display_proxy.get_registry();

    // instantiate a global with wrong version, this should kill the client

    registry.bind::<WlCompositor>(2, 1);
    assert!(roundtrip(&mut client, &mut server).is_err());
}

#[test]
fn invalid_global_version() {
    use wayc::protocol::wl_compositor::WlCompositor;

    let mut server = TestServer::new();
    server
        .display
        .create_global::<ServerCompositor, _>(1, ways::Filter::new(|_: (_, _), _, _| {}));

    let mut client = TestClient::new(&server.socket_name);
    let registry = client.display_proxy.get_registry();

    // instantiate a global with version 0, which is invalid this should kill the client

    registry.bind::<WlCompositor>(0, 1);

    assert!(roundtrip(&mut client, &mut server).is_err());
}

#[test]
fn wrong_global_id() {
    use wayc::protocol::wl_compositor::WlCompositor;

    let mut server = TestServer::new();
    server
        .display
        .create_global::<ServerCompositor, _>(1, ways::Filter::new(|_: (_, _), _, _| {}));

    let mut client = TestClient::new(&server.socket_name);
    let registry = client.display_proxy.get_registry();

    // instantiate a global with wrong id, this should kill the client

    registry.bind::<WlCompositor>(1, 3);

    assert!(roundtrip(&mut client, &mut server).is_err());
}

#[test]
fn two_step_binding() {
    use wayc::protocol::wl_compositor::WlCompositor;
    use wayc::protocol::wl_output::WlOutput;

    let mut server = TestServer::new();
    server
        .display
        .create_global::<ServerCompositor, _>(1, ways::Filter::new(|_: (_, _), _, _| {}));

    let mut client = TestClient::new(&server.socket_name);
    let manager = wayc::GlobalManager::new(&client.display_proxy);

    roundtrip(&mut client, &mut server).unwrap();

    // add a new global while clients already exist
    server
        .display
        .create_global::<ServerOutput, _>(1, ways::Filter::new(|_: (_, _), _, _| {}));

    roundtrip(&mut client, &mut server).unwrap();

    manager.instantiate_exact::<WlCompositor>(1).unwrap();

    manager.instantiate_exact::<WlOutput>(1).unwrap();

    roundtrip(&mut client, &mut server).unwrap();
}
