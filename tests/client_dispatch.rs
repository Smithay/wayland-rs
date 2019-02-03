extern crate calloop;

mod helpers;

use helpers::{wayc, ways, TestClient};

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
        let mut event_loop = ways::calloop::EventLoop::<()>::new().unwrap();
        let mut display = ways::Display::new(event_loop.handle());
        display.add_socket(Some(socket_name)).unwrap();

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

    let mut client = TestClient::new(OsStr::new(socket_name));

    client.event_queue.sync_roundtrip().unwrap();

    *(kill_switch.lock().unwrap()) = true;

    server_thread.join().unwrap();
}

#[test]
fn client_dispatch() {
    let socket_name = "wayland-client-dispatch";

    let kill_switch = Arc::new(Mutex::new(false));
    let server_kill_switch = kill_switch.clone();

    let server_thread = ::std::thread::spawn(move || {
        let mut event_loop = ways::calloop::EventLoop::<()>::new().unwrap();
        let mut display = ways::Display::new(event_loop.handle());
        display.add_socket(Some(socket_name)).unwrap();

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

    let mut client = TestClient::new(OsStr::new(socket_name));

    // do a manual roundtrip
    let done = Rc::new(Cell::new(false));
    let done2 = done.clone();
    client
        .display
        .sync(move |newcb| newcb.implement_closure(move |_, _| done2.set(true), ()))
        .unwrap();
    while !done.get() {
        client.event_queue.dispatch().unwrap();
    }

    *(kill_switch.lock().unwrap()) = true;

    server_thread.join().unwrap();
}

#[test]
fn client_sink() {
    let socket_name = "wayland-client-sink";

    let kill_switch = Arc::new(Mutex::new(false));
    let server_kill_switch = kill_switch.clone();

    let server_thread = ::std::thread::spawn(move || {
        let mut event_loop = ways::calloop::EventLoop::<()>::new().unwrap();
        let mut display = ways::Display::new(event_loop.handle());
        display.add_socket(Some(socket_name)).unwrap();

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

    let mut client = TestClient::new(OsStr::new(socket_name));

    // do a manual roundtrip
    let done = Rc::new(Cell::new(false));
    let done2 = done.clone();
    client
        .display
        .sync(move |newcb| newcb.implement_closure(move |_, _| done2.set(true), ()))
        .unwrap();
    while !done.get() {
        client.event_queue.dispatch().unwrap();
    }

    let (sink, msgiter) = wayc::sinks::blocking_message_iterator(client.event_queue.get_token());

    client
        .display
        .sync(move |newcb| newcb.implement(sink, ()))
        .unwrap();

    for (evt, _) in msgiter {
        match evt {
            wayc::protocol::wl_callback::Event::Done { .. } => break,
            _ => unreachable!(),
        }
    }

    *(kill_switch.lock().unwrap()) = true;

    server_thread.join().unwrap();
}

#[test]
fn client_calloop() {
    let socket_name = "wayland-calloop";

    let kill_switch = Arc::new(Mutex::new(false));
    let server_kill_switch = kill_switch.clone();

    let server_thread = ::std::thread::spawn(move || {
        let mut event_loop = ways::calloop::EventLoop::<()>::new().unwrap();
        let mut display = ways::Display::new(event_loop.handle());
        display.add_socket(Some(socket_name)).unwrap();

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

    let mut evl = calloop::EventLoop::new().unwrap();

    let _source = evl
        .handle()
        .insert_source(client.event_queue, |(), &mut ()| {})
        .unwrap();

    let done = Rc::new(Cell::new(false));
    let done2 = done.clone();
    client
        .display
        .sync(move |newcb| newcb.implement_closure(move |_, _| done2.set(true), ()))
        .unwrap();
    while !done.get() {
        client.display.flush().unwrap();
        evl.dispatch(None, &mut ()).unwrap();
    }

    *(kill_switch.lock().unwrap()) = true;

    server_thread.join().unwrap();
}
