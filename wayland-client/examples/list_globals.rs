use std::sync::{atomic::AtomicBool, Arc};

use wayland_client::{
    backend::WaylandError,
    convert_event,
    protocol::{wl_callback, wl_display, wl_registry},
    proxy_internals::ProxyData,
    Connection,
};

fn main() {
    let cx = Connection::connect_to_env().unwrap();

    let display = cx.handle().display();

    let registry_data = Some(Arc::new(ProxyData::new(Arc::new(|_handle, msg| {
        let (_registry, event) = convert_event!(msg; wl_registry::WlRegistry).unwrap();
        eprintln!("{:?}", event);
    }))));

    cx.handle().send_request(&display, wl_display::Request::GetRegistry {}, registry_data).unwrap();

    let done = Arc::new(AtomicBool::new(false));
    let done2 = done.clone();

    let callback_data = Some(Arc::new(ProxyData::new(Arc::new(move |_handle, msg| {
        let (_callback, event) = convert_event!(msg; wl_callback::WlCallback).unwrap();
        eprintln!("{:?}", event);
        done2.store(true, std::sync::atomic::Ordering::SeqCst);
    }))));

    cx.handle().send_request(&display, wl_display::Request::Sync {}, callback_data).unwrap();

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
