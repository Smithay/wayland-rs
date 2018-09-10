mod helpers;

use helpers::{roundtrip, wayc, ways, TestClient, TestServer};

use ways::protocol::{wl_compositor, wl_output, wl_shm};

struct Privilegied;

#[test]
fn global_filter() {
    use std::os::unix::io::IntoRawFd;

    let mut server = TestServer::new();

    // everyone see the compositor
    server
        .display
        .create_global::<wl_compositor::WlCompositor, _>(1, |_, _| {});

    // everyone see the shm
    server
        .display
        .create_global_with_filter::<wl_shm::WlShm, _, _>(1, |_, _| {}, |_| true);

    // only privilegied clients see the output
    let privilegied_output = server
        .display
        .create_global_with_filter::<wl_output::WlOutput, _, _>(
            1,
            |_, _| {},
            |client| client.data_map().get::<Privilegied>().is_some(),
        );

    // normal client only sees wo globals
    let mut client = TestClient::new(&server.socket_name);
    let manager = wayc::GlobalManager::new(&client.display);

    roundtrip(&mut client, &mut server).unwrap();

    assert_eq!(manager.list().len(), 2);

    let (server_cx, client_cx) = ::std::os::unix::net::UnixStream::pair().unwrap();

    let priv_client = unsafe { server.display.create_client(server_cx.into_raw_fd()) };
    priv_client.data_map().insert_if_missing(|| Privilegied);

    let mut client2 = unsafe { TestClient::from_fd(client_cx.into_raw_fd()) };
    let manager2 = wayc::GlobalManager::new(&client2.display);

    roundtrip(&mut client2, &mut server).unwrap();

    assert_eq!(manager2.list().len(), 3);

    // now destroy the privilegied globa
    // only privilegied clients will receive the destroy event
    // if a regular client received it, it would panic as the server destroyed an
    // unknown global

    privilegied_output.destroy();

    roundtrip(&mut client, &mut server).unwrap();
    roundtrip(&mut client2, &mut server).unwrap();

    assert_eq!(manager.list().len(), 2);
    assert_eq!(manager2.list().len(), 2);
}

#[test]
fn global_filter_try_force() {
    use wayc::protocol::wl_display::RequestsTrait as DisplayRequests;
    use wayc::protocol::wl_output::WlOutput;
    use wayc::protocol::wl_registry::RequestsTrait as RegistryRequests;

    use std::os::unix::io::IntoRawFd;

    let mut server = TestServer::new();

    // only privilegied clients see the output
    server
        .display
        .create_global_with_filter::<wl_output::WlOutput, _, _>(
            1,
            |_, _| {},
            |client| client.data_map().get::<Privilegied>().is_some(),
        );

    // normal client that cannot bind the privilegied global
    let mut client = TestClient::new(&server.socket_name);

    // privilegied client that can
    let (server_cx, client_cx) = ::std::os::unix::net::UnixStream::pair().unwrap();

    let priv_client = unsafe { server.display.create_client(server_cx.into_raw_fd()) };
    priv_client.data_map().insert_if_missing(|| Privilegied);

    let mut client2 = unsafe { TestClient::from_fd(client_cx.into_raw_fd()) };

    // privilegied client can bind it

    let registry2 = client2
        .display
        .get_registry(|newp| newp.implement(|_, _| {}, ()))
        .unwrap();
    registry2
        .bind::<WlOutput, _>(1, 1, |newp| newp.implement(|_, _| {}, ()))
        .unwrap();

    roundtrip(&mut client2, &mut server).unwrap();

    // unprivilegied client cannot

    let registry = client
        .display
        .get_registry(|newp| newp.implement(|_, _| {}, ()))
        .unwrap();
    registry
        .bind::<WlOutput, _>(1, 1, |newp| newp.implement(|_, _| {}, ()))
        .unwrap();

    assert!(roundtrip(&mut client, &mut server).is_err());
}
