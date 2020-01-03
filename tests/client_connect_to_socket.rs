mod helpers;

use helpers::{roundtrip, wayc, ways, TestClient, TestServer};

use ways::protocol::wl_output::WlOutput as ServerOutput;

use std::os::unix::io::IntoRawFd;

fn main() {
    let mut server = TestServer::new();
    server.display.create_global::<ServerOutput, _>(2, |_, _, _| {});

    let (s1, s2) = ::std::os::unix::net::UnixStream::pair().unwrap();

    let my_client = unsafe { server.display.create_client(s1.into_raw_fd(), &mut ()) };

    let fd2 = s2.into_raw_fd();
    ::std::env::set_var("WAYLAND_SOCKET", format!("{}", fd2));

    let mut client = TestClient::new_auto();
    let manager = wayc::GlobalManager::new(&client.display_proxy);

    roundtrip(&mut client, &mut server).unwrap();
    // check that we connected to the right compositor
    let globals = manager.list();
    assert!(globals.len() == 1);
    assert_eq!(globals[0], (1, "wl_output".into(), 2));

    my_client.kill();

    assert!(roundtrip(&mut client, &mut server).is_err());
}
