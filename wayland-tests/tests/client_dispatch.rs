mod helpers;

use helpers::{ways, TestClient};

use std::cell::Cell;
use std::ffi::OsStr;
use std::rc::Rc;
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

#[test]
fn client_sync_roundtrip() {
    let socket_name = "wayland-client-sync-roundtrip";

    let kill_switch = Arc::new(Mutex::new(false));
    let server_kill_switch = kill_switch.clone();

    let server_startup_info = Arc::new((Mutex::new(false), Condvar::new()));
    let server_startup_info_clone = server_startup_info.clone();

    let server_thread = ::std::thread::spawn(move || {
        let mut display = ways::Display::new();
        display.add_socket(Some(socket_name)).unwrap();

        // Make sure to release the lock.
        {
            let (lock, cvar) = &*server_startup_info_clone;
            let mut started = lock.lock().unwrap();
            *started = true;
            // Notify the client that we're ready.
            cvar.notify_one();
        }

        loop {
            display.dispatch(Duration::from_millis(100), &mut ()).unwrap();
            display.flush_clients(&mut ());
            if *(server_kill_switch.lock().unwrap()) {
                break;
            }
        }
    });

    // Wait for the server to start up.
    let (lock, cvar) = &*server_startup_info;
    let mut started = lock.lock().unwrap();
    while !*started {
        started = cvar.wait(started).unwrap();
    }

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

    let server_startup_info = Arc::new((Mutex::new(false), Condvar::new()));
    let server_startup_info_clone = server_startup_info.clone();

    let server_thread = ::std::thread::spawn(move || {
        let mut display = ways::Display::new();
        display.add_socket(Some(socket_name)).unwrap();

        // Make sure to release the lock.
        {
            let (lock, cvar) = &*server_startup_info_clone;
            let mut started = lock.lock().unwrap();
            *started = true;
            // Notify the client that we're ready.
            cvar.notify_one();
        }

        loop {
            display.dispatch(Duration::from_millis(100), &mut ()).unwrap();
            display.flush_clients(&mut ());
            if *(server_kill_switch.lock().unwrap()) {
                break;
            }
        }
    });

    // Wait for the server to start up.
    let (lock, cvar) = &*server_startup_info;
    let mut started = lock.lock().unwrap();
    while !*started {
        started = cvar.wait(started).unwrap();
    }

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
