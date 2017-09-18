#[macro_use]
extern crate wayland_client;

use wayland_client::EnvHandler;

wayland_env!(WaylandEnv);

fn main() {
    let (display, mut event_queue) = match wayland_client::default_connect() {
        Ok(ret) => ret,
        Err(e) => panic!("Cannot connect to wayland server: {:?}", e),
    };

    let registry = display.get_registry();

    let env_token = EnvHandler::<WaylandEnv>::init(&mut event_queue, &registry);

    event_queue.sync_roundtrip().unwrap();

    let state = event_queue.state();

    let env = state.get(&env_token);

    println!("Globals advertised by server:");
    for &(name, ref interface, version) in env.globals() {
        println!("{:4} : {} (version {})", name, interface, version);
    }
}
