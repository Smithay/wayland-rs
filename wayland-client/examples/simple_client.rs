#[macro_use]
extern crate wayland_client;

use wayland_client::EnvHandler;

wayland_env!(WaylandEnv);

fn main() {
    let (display, mut event_queue) = match wayland_client::default_connect() {
        Ok(ret) => ret,
        Err(e) => panic!("Cannot connect to wayland server: {:?}", e),
    };

    event_queue.add_handler(EnvHandler::<WaylandEnv>::new());

    let registry = display.get_registry();
    event_queue.register::<_, EnvHandler<WaylandEnv>>(&registry, 0);

    event_queue.sync_roundtrip().unwrap();

    let state = event_queue.state();

    let env = state.get_handler::<EnvHandler<WaylandEnv>>(0);

    println!("Globals advertised by server:");
    for &(name, ref interface, version) in env.globals() {
        println!("{:4} : {} (version {})", name, interface, version);
    }
}
