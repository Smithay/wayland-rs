mod helpers;

use helpers::{wayc, ways, TestClient};

use ways::protocol::wl_seat::WlSeat as ServerSeat;

use wayc::protocol::wl_seat;

use std::ffi::OsStr;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::Duration;

#[test]
fn display_to_new_thread() {
    let socket_name = "wayland-client-display-to-new-thread";

    let kill_switch = Arc::new(Mutex::new(false));
    let server_kill_switch = kill_switch.clone();

    let server_startup_info = Arc::new((Mutex::new(false), Condvar::new()));
    let server_startup_info_clone = server_startup_info.clone();

    let server_thread = thread::spawn(move || {
        let mut display = ways::Display::new();
        let socket = display.add_socket(Some(socket_name));

        // Make sure to release the lock.
        {
            let (lock, cvar) = &*server_startup_info_clone;
            let mut started = lock.lock().unwrap();
            *started = true;
            // Notify the client that we're ready.
            cvar.notify_one();
        }

        let _ = socket.unwrap();

        display.create_global::<ServerSeat, _>(5, ways::Filter::new(|_: (_, _), _, _| {}));

        loop {
            display.dispatch(Duration::from_millis(10), &mut ()).unwrap();
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

    let client = TestClient::new(OsStr::new(socket_name));

    let display_clone = client.display.clone();

    thread::spawn(move || {
        let mut evq = display_clone.create_event_queue();
        let attached = (**display_clone).clone().attach(evq.token());
        let manager = wayc::GlobalManager::new(&attached);
        evq.sync_roundtrip(&mut (), |_, _, _| unreachable!()).unwrap();
        manager.instantiate_exact::<wl_seat::WlSeat>(5).unwrap();
    })
    .join()
    .unwrap();

    *kill_switch.lock().unwrap() = true;

    server_thread.join().unwrap();
}

#[test]
#[cfg(feature = "client_native")]
fn display_from_external_on_new_thread() {
    let socket_name = "wayland-client-display-to-new-thread-external";

    let kill_switch = Arc::new(Mutex::new(false));
    let server_kill_switch = kill_switch.clone();

    let server_startup_info = Arc::new((Mutex::new(false), Condvar::new()));
    let server_startup_info_clone = server_startup_info.clone();

    let server_thread = thread::spawn(move || {
        let mut display = ways::Display::new();
        let socket = display.add_socket(Some(socket_name));

        // Make sure to release the lock.
        {
            let (lock, cvar) = &*server_startup_info_clone;
            let mut started = lock.lock().unwrap();
            *started = true;
            // Notify the client that we're ready.
            cvar.notify_one();
        }

        let _ = socket.unwrap();

        display.create_global::<ServerSeat, _>(5, ways::Filter::new(|_: (_, _), _, _| {}));

        loop {
            display.dispatch(Duration::from_millis(10), &mut ()).unwrap();
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

    let client = TestClient::new(OsStr::new(socket_name));

    let display_ptr = unsafe { client.display.get_display_ptr().as_mut() }.unwrap();

    thread::spawn(move || {
        let display = unsafe { wayc::Display::from_external_display(display_ptr) };
        let mut evq = display.create_event_queue();
        let attached = (*display).clone().attach(evq.token());
        let manager = wayc::GlobalManager::new(&attached);
        evq.sync_roundtrip(&mut (), |_, _, _| {}).unwrap();
        manager.instantiate_exact::<wl_seat::WlSeat>(5).unwrap().quick_assign(|_, _, _| {});
    })
    .join()
    .unwrap();

    *kill_switch.lock().unwrap() = true;

    server_thread.join().unwrap();
}
