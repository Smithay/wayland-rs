extern crate wayland_client as wayc;
extern crate wayland_server as ways;

use ways::sources::EventSource;

mod helpers;

use helpers::TestServer;

#[test]
fn skel() {
    // Server setup
    //
    let mut server = TestServer::new();

    let idle = server
        .event_loop
        .add_idle_event_source(|_, idata| *idata = true, false);

    server.event_loop.dispatch(Some(1)).unwrap();

    let done = idle.remove();
    assert!(done);
}
