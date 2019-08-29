#[cfg(feature = "native_lib")]
#[macro_use]
extern crate wayland_sys;

mod helpers;

use helpers::{roundtrip, wayc, ways, TestClient, TestServer};

use ways::protocol::{wl_compositor, wl_output, wl_shm};

struct Privileged;

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

    // only privileged clients see the output
    let privileged_output = server
        .display
        .create_global_with_filter::<wl_output::WlOutput, _, _>(
            1,
            |_, _| {},
            |client| client.data_map().get::<Privileged>().is_some(),
        );

    // normal client only sees two globals
    let mut client = TestClient::new(&server.socket_name);
    let manager = wayc::GlobalManager::new(&client.display_proxy);

    roundtrip(&mut client, &mut server).unwrap();

    assert_eq!(manager.list().len(), 2);

    let (server_cx, client_cx) = ::std::os::unix::net::UnixStream::pair().unwrap();

    let priv_client = unsafe { server.display.create_client(server_cx.into_raw_fd()) };
    priv_client.data_map().insert_if_missing(|| Privileged);

    let mut client2 = unsafe { TestClient::from_fd(client_cx.into_raw_fd()) };
    let manager2 = wayc::GlobalManager::new(&client2.display_proxy);

    roundtrip(&mut client2, &mut server).unwrap();

    assert_eq!(manager2.list().len(), 3);

    // now destroy the privileged global
    // only privileged clients will receive the destroy event
    // if a regular client received it, it would panic as the server destroyed an
    // unknown global

    privileged_output.destroy();

    roundtrip(&mut client, &mut server).unwrap();
    roundtrip(&mut client2, &mut server).unwrap();

    assert_eq!(manager.list().len(), 2);
    assert_eq!(manager2.list().len(), 2);
}

#[test]
fn global_filter_try_force() {
    use wayc::protocol::wl_output::WlOutput;

    use std::os::unix::io::IntoRawFd;

    let mut server = TestServer::new();

    // only privileged clients see the output
    server
        .display
        .create_global_with_filter::<wl_output::WlOutput, _, _>(
            1,
            |_, _| {},
            |client| client.data_map().get::<Privileged>().is_some(),
        );

    // normal client that cannot bind the privileged global
    let mut client = TestClient::new(&server.socket_name);

    // privileged client that can
    let (server_cx, client_cx) = ::std::os::unix::net::UnixStream::pair().unwrap();

    let priv_client = unsafe { server.display.create_client(server_cx.into_raw_fd()) };
    priv_client.data_map().insert_if_missing(|| Privileged);

    let mut client2 = unsafe { TestClient::from_fd(client_cx.into_raw_fd()) };

    // privileged client can bind it

    let registry2 = client2.display_proxy.get_registry();
    registry2.bind::<WlOutput>(1, 1);

    roundtrip(&mut client2, &mut server).unwrap();

    // unprivileged client cannot

    let registry = client.display_proxy.get_registry();
    registry.bind::<WlOutput>(1, 1);

    assert!(roundtrip(&mut client, &mut server).is_err());
}

#[cfg(feature = "native_lib")]
#[test]
fn external_globals() {
    use std::os::raw::c_void;

    use helpers::ways::Interface;
    use wayland_sys::server::*;

    let mut server = TestServer::new();

    extern "C" fn dummy_global_bind(_client: *mut wl_client, _data: *mut c_void, _version: u32, _id: u32) {}

    // everyone see the compositor
    server
        .display
        .create_global::<wl_compositor::WlCompositor, _>(1, |_, _| {});

    // create a global via the C API, it'll not be initialized like a rust one
    unsafe {
        ffi_dispatch!(
            WAYLAND_SERVER_HANDLE,
            wl_global_create,
            server.display.c_ptr(),
            wl_shm::WlShm::c_interface(),
            1,
            ::std::ptr::null_mut(),
            dummy_global_bind
        );
    }

    // normal client only sees the two globals
    let mut client = TestClient::new(&server.socket_name);
    let manager = wayc::GlobalManager::new(&client.display);

    roundtrip(&mut client, &mut server).unwrap();

    assert_eq!(manager.list().len(), 2);
}
