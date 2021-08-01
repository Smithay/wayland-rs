use std::sync::{atomic::AtomicBool, Arc};

use wayland_client::{
    backend::WaylandError,
    convert_event,
    protocol::{wl_callback, wl_registry},
    proxy_internals::ProxyData,
    Connection,
};

fn main() {
    let cx = Connection::connect_to_env().unwrap();

    let display = cx.handle().display();

    let registry_data = ProxyData::new(Arc::new(|cx, msg| {
        let (_registry, event) = convert_event!(cx, msg; wl_registry::WlRegistry).unwrap();
        eprintln!("{:?}", event);
    }));

    let _registry = display.get_registry(&mut cx.handle(), Some(registry_data)).unwrap();

    let done = Arc::new(AtomicBool::new(false));
    let done2 = done.clone();

    let callback_data = ProxyData::new(Arc::new(move |cx, msg| {
        let (_callback, event) = convert_event!(cx, msg; wl_callback::WlCallback).unwrap();
        eprintln!("{:?}", event);
        done2.store(true, std::sync::atomic::Ordering::SeqCst);
    }));

    let _callback = display.sync(&mut cx.handle(), Some(callback_data)).unwrap();
    cx.flush().unwrap();

    while !done.load(std::sync::atomic::Ordering::SeqCst) {
        match cx.dispatch_events() {
            Ok(_) => {}
            Err(WaylandError::Protocol(e)) => panic!("Protocol error: {:?}", e),
            Err(WaylandError::Io(e)) => {
                if e.kind() == std::io::ErrorKind::WouldBlock {
                    continue;
                } else {
                    panic!("IO error: {:?}", e);
                }
            }
        }
    }
}
