mod helpers;

use helpers::*;

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

#[test]
fn client_roundtrip() {
    let kill_switch = Arc::new(AtomicBool::new(false));
    let server_kill_switch = kill_switch.clone();

    let mut server = TestServer::new();

    let (_, client) = server.add_client::<()>();

    let server_thread = ::std::thread::spawn(move || loop {
        server.display.dispatch_clients(&mut ()).unwrap();
        server.display.flush_clients().unwrap();
        if server_kill_switch.load(Ordering::Acquire) {
            break;
        }
    });

    client.conn.roundtrip().unwrap();

    kill_switch.store(true, Ordering::Release);

    server_thread.join().unwrap();
}
