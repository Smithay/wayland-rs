mod helpers;

use helpers::{roundtrip, wayc, ways, TestClient, TestServer};

use ways::protocol::{wl_compositor, wl_output};

use wayc::protocol::wl_compositor::WlCompositor as ClientCompositor;
use wayc::protocol::wl_output::WlOutput as ClientOutput;

use std::sync::{Arc, Mutex};

#[test]
fn client_user_data() {
    let mut server = TestServer::new();
    let clients = Arc::new(Mutex::new(Vec::new()));

    struct HasOutput;
    struct HasCompositor;

    server.display.create_global::<wl_output::WlOutput, _>(1, {
        let clients = clients.clone();
        ways::Filter::new(move |(output, _): (ways::Main<wl_output::WlOutput>, u32), _, _| {
            let client = output.as_ref().client().unwrap();
            let ret = client.data_map().insert_if_missing(|| HasOutput);
            // the data should not be already here
            assert!(ret);
            clients.lock().unwrap().push(client);
        })
    });
    server.display.create_global::<wl_compositor::WlCompositor, _>(1, {
        let clients = clients.clone();
        ways::Filter::new(
            move |(compositor, _): (ways::Main<wl_compositor::WlCompositor>, u32), _, _| {
                let client = compositor.as_ref().client().unwrap();
                let ret = client.data_map().insert_if_missing(|| HasCompositor);
                // the data should not be already here
                assert!(ret);
                clients.lock().unwrap().push(client);
            },
        )
    });

    let mut client = TestClient::new(&server.socket_name);
    let manager = wayc::GlobalManager::new(&client.display_proxy);

    roundtrip(&mut client, &mut server).unwrap();

    // Instantiate the globals
    manager.instantiate_exact::<ClientOutput>(1).unwrap();

    roundtrip(&mut client, &mut server).unwrap();

    {
        let clients = clients.lock().unwrap();

        assert!(clients.len() == 1);
        assert!(clients[0].data_map().get::<HasOutput>().is_some());
        assert!(clients[0].data_map().get::<HasCompositor>().is_none());
    }

    manager.instantiate_exact::<ClientCompositor>(1).unwrap();

    roundtrip(&mut client, &mut server).unwrap();

    let clients = clients.lock().unwrap();

    assert!(clients.len() == 2);
    assert!(clients[0].equals(&clients[1]));
    assert!(clients[0].data_map().get::<HasCompositor>().is_some());
    assert!(clients[0].data_map().get::<HasOutput>().is_some());
    assert!(clients[1].data_map().get::<HasCompositor>().is_some());
    assert!(clients[1].data_map().get::<HasOutput>().is_some());
}

#[test]
fn client_credentials() {
    let mut server = TestServer::new();

    let server_client = Arc::new(Mutex::new(None));

    server.display.create_global::<wl_output::WlOutput, _>(1, {
        let server_client = server_client.clone();
        ways::Filter::new(move |(output, _): (ways::Main<wl_output::WlOutput>, u32), _, _| {
            let client = output.as_ref().client().unwrap();
            server_client.lock().unwrap().replace(client);
        })
    });
    let mut client = TestClient::new(&server.socket_name);
    let manager = wayc::GlobalManager::new(&client.display_proxy);

    roundtrip(&mut client, &mut server).unwrap();

    // Instantiate the globals
    manager.instantiate_exact::<ClientOutput>(1).unwrap();

    roundtrip(&mut client, &mut server).unwrap();

    let server_client = server_client.lock().unwrap().take().unwrap();
    let credentials = server_client.credentials();
    assert!(credentials.is_some());
    assert_credentials(credentials.unwrap());
}

#[cfg(any(not(feature = "server_native"), not(target_os = "freebsd")))]
fn assert_credentials(credentials: ways::Credentials) {
    assert!(credentials.pid != 0);
}

#[cfg(all(feature = "server_native", target_os = "freebsd"))]
fn assert_credentials(_credentials: ways::Credentials) {
    // The current implementation of wl_client_get_credentials
    // will always return pid == 0 on freebsd
    // On recent versions this has been fixed with a freebsd
    // specific patch. Detecting if a patched version is used
    // is too complicated and this assert would just test the
    // native wayland-server library. So the assert is a no-op
    // for now.
    //
    // see: https://bugs.freebsd.org/bugzilla/show_bug.cgi?id=246189
}
