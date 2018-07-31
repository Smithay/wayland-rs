mod helpers;

use helpers::{roundtrip, wayc, ways, TestClient, TestServer};

use std::ffi::OsStr;
use std::sync::{Arc, Mutex};

#[test]
fn client_sync_roundtrip() {
    let socket_name = "wayland-client-dispatch";

    let kill_switch = Arc::new(Mutex::new(false));
    let server_kill_switch = kill_switch.clone();

    let server_thread = ::std::thread::spawn(move || {
        let (mut display, mut event_loop) = ways::Display::new();
        display.add_socket(Some(socket_name)).unwrap();

        loop {
            event_loop.dispatch(Some(10)).unwrap();
            display.flush_clients();
            if *(server_kill_switch.lock().unwrap()) {
                break;
            }
        }
    });

    // let the server boot up
    ::std::thread::sleep(::std::time::Duration::from_millis(100));

    let mut client = TestClient::new(OsStr::new(socket_name));

    client.event_queue.sync_roundtrip().unwrap();

    *(kill_switch.lock().unwrap()) = true;

    server_thread.join().unwrap();
}
