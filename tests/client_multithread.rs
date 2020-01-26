mod helpers;

use helpers::{wayc, ways, TestClient};

use ways::protocol::wl_seat::WlSeat as ServerSeat;

use wayc::protocol::wl_seat;

use std::ffi::OsStr;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

#[test]
fn display_to_new_thread() {
    let socket_name = "wayland-client-display-to-new-thread";

    let kill_switch = Arc::new(Mutex::new(false));
    let server_kill_switch = kill_switch.clone();

    let server_thread = thread::spawn(move || {
        let mut display = ways::Display::new();
        display.add_socket(Some(socket_name)).unwrap();
        display.create_global::<ServerSeat, _>(5, ways::Filter::new(|_: (_, _), _, _| {}));

        loop {
            display.dispatch(Duration::from_millis(10), &mut ()).unwrap();
            display.flush_clients(&mut ());
            if *(server_kill_switch.lock().unwrap()) {
                break;
            }
        }
    });

    // let the server boot up
    ::std::thread::sleep(::std::time::Duration::from_millis(100));

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

    let server_thread = thread::spawn(move || {
        let mut display = ways::Display::new();
        display.add_socket(Some(socket_name)).unwrap();
        display.create_global::<ServerSeat, _>(5, ways::Filter::new(|_: (_, _), _, _| {}));

        loop {
            display.dispatch(Duration::from_millis(10), &mut ()).unwrap();
            display.flush_clients(&mut ());
            if *(server_kill_switch.lock().unwrap()) {
                break;
            }
        }
    });

    // let the server boot up
    ::std::thread::sleep(::std::time::Duration::from_millis(100));

    let client = TestClient::new(OsStr::new(socket_name));

    let display_ptr = unsafe { client.display.get_display_ptr().as_mut() }.unwrap();

    thread::spawn(move || {
        let display = unsafe { wayc::Display::from_external_display(display_ptr) };
        let mut evq = display.create_event_queue();
        let attached = (*display).clone().attach(evq.token());
        let manager = wayc::GlobalManager::new(&attached);
        evq.sync_roundtrip(&mut (), |_, _, _| {}).unwrap();
        manager
            .instantiate_exact::<wl_seat::WlSeat>(5)
            .unwrap()
            .quick_assign(|_, _, _| {});
    })
    .join()
    .unwrap();

    *kill_switch.lock().unwrap() = true;

    server_thread.join().unwrap();
}
