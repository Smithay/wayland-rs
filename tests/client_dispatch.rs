mod helpers;

use helpers::{ways, TestClient};

use std::cell::Cell;
use std::ffi::OsStr;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[test]
fn client_sync_roundtrip() {
    let socket_name = "wayland-client-sync-roundtrip";

    let kill_switch = Arc::new(Mutex::new(false));
    let server_kill_switch = kill_switch.clone();

    let server_thread = ::std::thread::spawn(move || {
        let mut display = ways::Display::new();
        display.add_socket(Some(socket_name)).unwrap();

        loop {
            display.dispatch(Duration::from_millis(100), &mut ()).unwrap();
            display.flush_clients(&mut ());
            if *(server_kill_switch.lock().unwrap()) {
                break;
            }
        }
    });

    // let the server boot up
    ::std::thread::sleep(::std::time::Duration::from_millis(500));

    let mut client = TestClient::new(OsStr::new(socket_name));

    client.event_queue.sync_roundtrip(&mut (), |_, _, _| unreachable!()).unwrap();

    *(kill_switch.lock().unwrap()) = true;

    server_thread.join().unwrap();
}

#[test]
fn client_dispatch() {
    let socket_name = "wayland-client-dispatch";

    let kill_switch = Arc::new(Mutex::new(false));
    let server_kill_switch = kill_switch.clone();

    let server_thread = ::std::thread::spawn(move || {
        let mut display = ways::Display::new();
        display.add_socket(Some(socket_name)).unwrap();

        loop {
            display.dispatch(Duration::from_millis(100), &mut ()).unwrap();
            display.flush_clients(&mut ());
            if *(server_kill_switch.lock().unwrap()) {
                break;
            }
        }
    });

    // let the server boot up
    ::std::thread::sleep(::std::time::Duration::from_millis(500));

    let mut client = TestClient::new(OsStr::new(socket_name));

    // do a manual roundtrip
    let done = Rc::new(Cell::new(false));
    let done2 = done.clone();
    client.display_proxy.sync().quick_assign(move |_, _, _| done2.set(true));
    while !done.get() {
        client.event_queue.dispatch(&mut (), |_, _, _| unreachable!()).unwrap();
    }

    *(kill_switch.lock().unwrap()) = true;

    server_thread.join().unwrap();
}
