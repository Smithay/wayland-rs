#[macro_use]
mod helpers;

use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::sync_channel,
    Arc,
};

use helpers::{wayc, ways, TestServer};

use ways::protocol::wl_compositor::WlCompositor as ServerCompositor;
use ways::protocol::wl_output::WlOutput as ServerOutput;
use ways::protocol::wl_shell::WlShell as ServerShell;

use wayc::globals::{registry_queue_init, Global, GlobalListContents};
use wayc::protocol::{wl_compositor, wl_registry, wl_subcompositor};

#[test]
fn client_global_helpers_init() {
    let kill_switch = Arc::new(AtomicBool::new(false));
    let server_kill_switch = kill_switch.clone();

    let mut server = TestServer::new();
    server.display.handle().create_global::<ServerHandler, ServerCompositor, _>(4, ());
    server.display.handle().create_global::<ServerHandler, ServerOutput, _>(2, ());
    server.display.handle().create_global::<ServerHandler, ServerShell, _>(1, ());

    let (_, client) = server.add_client::<()>();

    // spawn a thread for the server loop as the client global init helpers to a blocking roundtrip
    let server_thread = ::std::thread::spawn(move || loop {
        server.display.dispatch_clients(&mut ServerHandler).unwrap();
        server.display.flush_clients().unwrap();
        if server_kill_switch.load(Ordering::Acquire) {
            break;
        }
    });

    let (globals, queue) = registry_queue_init::<ClientHandler>(&client.conn).unwrap();

    assert_eq!(
        globals.contents().clone_list(),
        &[
            Global { name: 1, interface: "wl_compositor".into(), version: 4 },
            Global { name: 2, interface: "wl_output".into(), version: 2 },
            Global { name: 3, interface: "wl_shell".into(), version: 1 },
        ]
    );

    // ensure bind works as expected
    // Too high version fails
    assert!(globals.bind::<wl_compositor::WlCompositor, _, _>(&queue.handle(), 5..=5, ()).is_err());
    // Missing global fails
    assert!(globals
        .bind::<wl_subcompositor::WlSubcompositor, _, _>(&queue.handle(), 1..=1, ())
        .is_err());
    // Compatible spec succeeds
    assert!(globals.bind::<wl_compositor::WlCompositor, _, _>(&queue.handle(), 1..=5, ()).is_ok());

    // cleanup
    kill_switch.store(true, Ordering::Release);
    server_thread.join().unwrap();
}

#[test]
fn client_global_helpers_dynamic() {
    let kill_switch = Arc::new(AtomicBool::new(false));
    let server_kill_switch = kill_switch.clone();

    let mut server = TestServer::new();
    server.display.handle().create_global::<ServerHandler, ServerCompositor, _>(4, ());
    server.display.handle().create_global::<ServerHandler, ServerShell, _>(1, ());

    let (_, client) = server.add_client::<()>();

    let (tx, rx) = sync_channel(0);

    // spawn a thread for the server loop as the client global init helpers to a blocking roundtrip
    let server_thread = ::std::thread::spawn(move || {
        let mut output = None;
        loop {
            if let Ok(()) = rx.try_recv() {
                if let Some(id) = output.take() {
                    server.display.handle().remove_global::<ServerHandler>(id);
                } else {
                    // create the global
                    output = Some(
                        server
                            .display
                            .handle()
                            .create_global::<ServerHandler, ServerOutput, _>(2, ()),
                    );
                }
            }
            server.display.dispatch_clients(&mut ServerHandler).unwrap();
            server.display.flush_clients().unwrap();
            if server_kill_switch.load(Ordering::Acquire) {
                break;
            }
        }
    });

    let (globals, mut queue) = registry_queue_init::<ClientHandler>(&client.conn).unwrap();

    assert_eq!(
        globals.contents().clone_list(),
        &[
            Global { name: 1, interface: "wl_compositor".into(), version: 4 },
            Global { name: 2, interface: "wl_shell".into(), version: 1 },
        ]
    );

    // create the wl_output
    tx.send(()).unwrap();

    let mut state = ClientHandler(false);
    queue.blocking_dispatch(&mut state).unwrap();
    assert!(state.0);
    assert_eq!(
        globals.contents().clone_list(),
        &[
            Global { name: 1, interface: "wl_compositor".into(), version: 4 },
            Global { name: 2, interface: "wl_shell".into(), version: 1 },
            Global { name: 3, interface: "wl_output".into(), version: 2 },
        ]
    );

    // destroy the wl_output
    tx.send(()).unwrap();

    let mut state = ClientHandler(false);
    queue.blocking_dispatch(&mut state).unwrap();
    assert!(!state.0);
    assert_eq!(
        globals.contents().clone_list(),
        &[
            Global { name: 1, interface: "wl_compositor".into(), version: 4 },
            Global { name: 2, interface: "wl_shell".into(), version: 1 },
        ]
    );

    // cleanup
    kill_switch.store(true, Ordering::Release);
    server_thread.join().unwrap();
}

#[test]
#[should_panic]
fn too_high_global_version() {
    let kill_switch = Arc::new(AtomicBool::new(false));
    let server_kill_switch = kill_switch.clone();

    let mut server = TestServer::new();
    server.display.handle().create_global::<ServerHandler, ServerCompositor, _>(4, ());

    let (_, client) = server.add_client::<()>();

    // spawn a thread for the server loop as the client global init helpers to a blocking roundtrip
    let server_thread = ::std::thread::spawn(move || loop {
        server.display.dispatch_clients(&mut ServerHandler).unwrap();
        server.display.flush_clients().unwrap();
        if server_kill_switch.load(Ordering::Acquire) {
            break;
        }
    });

    let (globals, queue) = registry_queue_init::<ClientHandler>(&client.conn).unwrap();

    // kill the server now to avoit a deadlock of the test, as we're about to panic
    kill_switch.store(true, Ordering::Release);
    server_thread.join().unwrap();

    let max_compositor_version =
        <wl_compositor::WlCompositor as wayland_client::Proxy>::interface().version;
    // invoking bind with too high a target version should panic
    let _ = globals.bind::<wl_compositor::WlCompositor, _, _>(
        &queue.handle(),
        1..=max_compositor_version + 1,
        (),
    );
}

struct ServerHandler;

server_ignore_impl!(ServerHandler => [ServerCompositor, ServerShell, ServerOutput]);
server_ignore_global_impl!(ServerHandler => [ServerCompositor, ServerShell, ServerOutput]);

struct ClientHandler(bool);

impl wayc::Dispatch<wl_registry::WlRegistry, GlobalListContents> for ClientHandler {
    fn event(
        state: &mut Self,
        _: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &GlobalListContents,
        _: &wayc::Connection,
        _: &wayc::QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global { name, interface, version } = event {
            assert_eq!(name, 3);
            assert_eq!(interface, "wl_output");
            assert_eq!(version, 2);
            state.0 = true;
        } else if let wl_registry::Event::GlobalRemove { name } = event {
            assert_eq!(name, 3);
            state.0 = false;
        } else {
            unreachable!()
        }
    }
}

client_ignore_impl!(ClientHandler => [
    wl_compositor::WlCompositor,
    wl_subcompositor::WlSubcompositor
]);
