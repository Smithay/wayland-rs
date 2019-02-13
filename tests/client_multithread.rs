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
        let mut event_loop = ways::calloop::EventLoop::<()>::new().unwrap();
        let mut display = ways::Display::new(event_loop.handle());
        display.add_socket(Some(socket_name)).unwrap();
        display.create_global::<ServerSeat, _>(5, |_, _| {});

        loop {
            event_loop
                .dispatch(Some(Duration::from_millis(10)), &mut ())
                .unwrap();
            display.flush_clients();
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
        let wrapper = (*display_clone).as_ref().make_wrapper(&evq.get_token()).unwrap();
        let manager = wayc::GlobalManager::new(&wrapper);
        evq.sync_roundtrip().unwrap();
        // can provide a non-send impl
        manager
            .instantiate_exact::<wl_seat::WlSeat, _>(5, |newp| newp.implement_closure(|_, _| {}, ()))
            .unwrap();
    })
    .join()
    .unwrap();

    *kill_switch.lock().unwrap() = true;

    server_thread.join().unwrap();
}

#[test]
#[cfg(feature = "native_lib")]
fn display_from_external_on_new_thread() {
    let socket_name = "wayland-client-display-to-new-thread-external";

    let kill_switch = Arc::new(Mutex::new(false));
    let server_kill_switch = kill_switch.clone();

    let server_thread = thread::spawn(move || {
        let mut event_loop = ways::calloop::EventLoop::<()>::new().unwrap();
        let mut display = ways::Display::new(event_loop.handle());
        display.add_socket(Some(socket_name)).unwrap();
        display.create_global::<ServerSeat, _>(5, |_, _| {});

        loop {
            event_loop
                .dispatch(Some(Duration::from_millis(10)), &mut ())
                .unwrap();
            display.flush_clients();
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
        let (wrapper, mut evq) = unsafe { wayc::Display::from_external_display(display_ptr) };
        let manager = wayc::GlobalManager::new(&wrapper);
        evq.sync_roundtrip().unwrap();
        // can provide a non-send impl
        manager
            .instantiate_exact::<wl_seat::WlSeat, _>(5, |newp| newp.implement_closure(|_, _| {}, ()))
            .unwrap();
    })
    .join()
    .unwrap();

    *kill_switch.lock().unwrap() = true;

    server_thread.join().unwrap();
}
