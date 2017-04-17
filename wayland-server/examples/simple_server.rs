#[macro_use]
extern crate wayland_server;


fn main() {
    let (display, mut event_loop) = wayland_server::create_display();

    event_loop.run();
}
