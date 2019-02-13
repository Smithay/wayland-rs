mod helpers;

use helpers::{roundtrip, wayc, ways, TestClient, TestServer};

use ways::protocol::wl_output::WlOutput as ServerOutput;

fn main() {
    let mut server = TestServer::new();
    server.display.create_global::<ServerOutput, _>(1, |_, _| {});

    ::std::env::remove_var("WAYLAND_DISPLAY");

    assert!(wayc::Display::connect_to_env().is_err());

    ::std::env::set_var("WAYLAND_DISPLAY", &server.socket_name);

    let mut client = TestClient::new_auto();
    let manager = wayc::GlobalManager::new(&client.display);

    roundtrip(&mut client, &mut server).unwrap();
    // check that we connected to the right compositor
    let globals = manager.list();
    assert!(globals.len() == 1);
    assert_eq!(globals[0], (1, "wl_output".into(), 1));
}
