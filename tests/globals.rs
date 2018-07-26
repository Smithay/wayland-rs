mod helpers;

use helpers::{roundtrip, wayc, ways, TestClient, TestServer};

use ways::protocol::wl_compositor::WlCompositor as ServerCompositor;
use ways::protocol::wl_output::WlOutput as ServerOutput;
use ways::protocol::wl_shell::WlShell as ServerShell;

use std::sync::{Arc, Mutex};

#[test]
fn simple_global() {
    let mut server = TestServer::new();
    let loop_token = server.event_loop.token();
    server
        .display
        .create_global::<ServerCompositor, _>(&loop_token, 1, |_, _| {});

    let mut client = TestClient::new(&server.socket_name);
    let manager = wayc::GlobalManager::new(&client.display);

    roundtrip(&mut client, &mut server).unwrap();
    let globals = manager.list();
    assert!(globals.len() == 1);
    assert_eq!(globals[0], (1, "wl_compositor".into(), 1));
}

#[test]
fn multi_versions() {
    let mut server = TestServer::new();
    let loop_token = server.event_loop.token();
    server
        .display
        .create_global::<ServerCompositor, _>(&loop_token, 4, |_, _| {});
    server
        .display
        .create_global::<ServerCompositor, _>(&loop_token, 3, |_, _| {});
    server
        .display
        .create_global::<ServerCompositor, _>(&loop_token, 2, |_, _| {});
    server
        .display
        .create_global::<ServerCompositor, _>(&loop_token, 1, |_, _| {});

    let mut client = TestClient::new(&server.socket_name);
    let manager = wayc::GlobalManager::new(&client.display);

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
    let loop_token = server.event_loop.token();
    server
        .display
        .create_global::<ServerCompositor, _>(&loop_token, 1, |_, _| {});

    let mut client = TestClient::new(&server.socket_name);
    let manager = wayc::GlobalManager::new(&client.display);

    roundtrip(&mut client, &mut server).unwrap();
    assert!(manager.list().len() == 1);

    server
        .display
        .create_global::<ServerShell, _>(&loop_token, 1, |_, _| {});

    roundtrip(&mut client, &mut server).unwrap();
    assert!(manager.list().len() == 2);

    let output = server
        .display
        .create_global::<ServerOutput, _>(&loop_token, 1, |_, _| {});

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
    let loop_token = server.event_loop.token();
    server
        .display
        .create_global::<ServerCompositor, _>(&loop_token, 1, |_, _| {});

    let counter = Arc::new(Mutex::new(0));
    let counter2 = counter.clone();

    let mut client = TestClient::new(&server.socket_name);
    let manager = wayc::GlobalManager::new_with_cb(&client.display, move |event, _| match event {
        GlobalEvent::New { .. } => *(counter2.lock().unwrap()) += 1,
        GlobalEvent::Removed { .. } => *(counter2.lock().unwrap()) -= 1,
    });

    roundtrip(&mut client, &mut server).unwrap();

    server
        .display
        .create_global::<ServerCompositor, _>(&loop_token, 1, |_, _| {});
    server
        .display
        .create_global::<ServerCompositor, _>(&loop_token, 1, |_, _| {});
    let comp = server
        .display
        .create_global::<ServerCompositor, _>(&loop_token, 1, |_, _| {});

    roundtrip(&mut client, &mut server).unwrap();

    assert!(manager.list().len() == 4);
    assert!(*counter.lock().unwrap() == 4);

    comp.destroy();

    roundtrip(&mut client, &mut server).unwrap();

    assert!(manager.list().len() == 3);
    assert!(*counter.lock().unwrap() == 3);
}

#[test]
fn auto_instanciate() {
    use wayc::protocol::wl_compositor::WlCompositor;
    use wayc::protocol::wl_output::WlOutput;
    use wayc::protocol::wl_shell::WlShell;
    use wayc::GlobalError;

    let mut server = TestServer::new();
    let loop_token = server.event_loop.token();
    server
        .display
        .create_global::<ServerCompositor, _>(&loop_token, 4, |_, _| {});
    server
        .display
        .create_global::<ServerShell, _>(&loop_token, 1, |_, _| {});

    let mut client = TestClient::new(&server.socket_name);
    let manager = wayc::GlobalManager::new(&client.display);

    roundtrip(&mut client, &mut server).unwrap();

    let compositor = manager
        .instantiate_auto::<WlCompositor, _>(|newp| newp.implement(|_, _| {}))
        .unwrap();
    assert!(compositor.version() == 4);
    let shell = manager
        .instantiate_auto::<WlShell, _>(|newp| newp.implement(|_, _| {}))
        .unwrap();
    assert!(shell.version() == 1);

    assert!(
        manager.instantiate_exact::<WlCompositor, _>(5, |newp| newp.implement(|_, _| {}))
            == Err(GlobalError::VersionTooLow(4))
    );
    assert!(
        manager.instantiate_exact::<WlOutput, _>(5, |newp| newp.implement(|_, _| {}))
            == Err(GlobalError::Missing)
    );
    assert!(
        manager.instantiate_auto::<WlOutput, _>(|newp| newp.implement(|_, _| {}))
            == Err(GlobalError::Missing)
    );
}

#[test]
#[should_panic]
fn wrong_version_create_global() {
    let mut server = TestServer::new();
    let loop_token = server.event_loop.token();
    server
        .display
        .create_global::<ServerCompositor, _>(&loop_token, 42, |_, _| {});
}

#[test]
#[cfg_attr(feature = "native_lib", ignore)]
fn wrong_global() {
    use wayc::protocol::wl_display::RequestsTrait as DisplayRequests;
    use wayc::protocol::wl_output::WlOutput;
    use wayc::protocol::wl_registry::RequestsTrait as RegistryRequests;

    let mut server = TestServer::new();
    let loop_token = server.event_loop.token();
    server
        .display
        .create_global::<ServerCompositor, _>(&loop_token, 1, |_, _| {});

    let mut client = TestClient::new(&server.socket_name);
    let registry = client
        .display
        .get_registry(|newp| newp.implement(|_, _| {}))
        .unwrap();

    // instanciate a wrong global, this should kill the client
    // But currently does not fail on native_lib

    registry
        .bind::<WlOutput, _>(1, 1, |newp| newp.implement(|_, _| {}))
        .unwrap();

    assert!(roundtrip(&mut client, &mut server).is_err());
}

#[test]
fn wrong_global_version() {
    use wayc::protocol::wl_compositor::WlCompositor;
    use wayc::protocol::wl_display::RequestsTrait as DisplayRequests;
    use wayc::protocol::wl_registry::RequestsTrait as RegistryRequests;

    let mut server = TestServer::new();
    let loop_token = server.event_loop.token();
    server
        .display
        .create_global::<ServerCompositor, _>(&loop_token, 1, |_, _| {});

    let mut client = TestClient::new(&server.socket_name);
    let registry = client
        .display
        .get_registry(|newp| newp.implement(|_, _| {}))
        .unwrap();

    // instanciate a global with wrong version, this shoudl kill the client

    registry
        .bind::<WlCompositor, _>(2, 1, |newp| newp.implement(|_, _| {}))
        .unwrap();

    assert!(roundtrip(&mut client, &mut server).is_err());
}

#[test]
fn invalid_global_version() {
    use wayc::protocol::wl_compositor::WlCompositor;
    use wayc::protocol::wl_display::RequestsTrait as DisplayRequests;
    use wayc::protocol::wl_registry::RequestsTrait as RegistryRequests;

    let mut server = TestServer::new();
    let loop_token = server.event_loop.token();
    server
        .display
        .create_global::<ServerCompositor, _>(&loop_token, 1, |_, _| {});

    let mut client = TestClient::new(&server.socket_name);
    let registry = client
        .display
        .get_registry(|newp| newp.implement(|_, _| {}))
        .unwrap();

    // instanciate a global with version 0, which is invalid this shoudl kill the client

    registry
        .bind::<WlCompositor, _>(0, 1, |newp| newp.implement(|_, _| {}))
        .unwrap();

    assert!(roundtrip(&mut client, &mut server).is_err());
}

#[test]
fn wrong_global_id() {
    use wayc::protocol::wl_compositor::WlCompositor;
    use wayc::protocol::wl_display::RequestsTrait as DisplayRequests;
    use wayc::protocol::wl_registry::RequestsTrait as RegistryRequests;

    let mut server = TestServer::new();
    let loop_token = server.event_loop.token();
    server
        .display
        .create_global::<ServerCompositor, _>(&loop_token, 1, |_, _| {});

    let mut client = TestClient::new(&server.socket_name);
    let registry = client
        .display
        .get_registry(|newp| newp.implement(|_, _| {}))
        .unwrap();

    // instanciate a global with wrong id, this should kill the client

    registry
        .bind::<WlCompositor, _>(1, 3, |newp| newp.implement(|_, _| {}))
        .unwrap();

    assert!(roundtrip(&mut client, &mut server).is_err());
}
