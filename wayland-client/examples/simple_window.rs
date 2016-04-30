extern crate byteorder;
extern crate tempfile;
#[macro_use]
extern crate wayland_client;

use byteorder::{WriteBytesExt, NativeEndian};

use std::io::Write;
use std::os::unix::io::AsRawFd;

use wayland_client::Proxy;
use wayland_client::wayland::get_display;
use wayland_client::wayland::compositor::WlCompositor;
use wayland_client::wayland::shell::WlShell;
use wayland_client::wayland::shm::{WlShm, WlShmFormat};

wayland_env!(WaylandEnv,
    compositor: WlCompositor,
    shell: WlShell,
    shm: WlShm
);

fn main() {
    let display = match get_display() {
        Some(d) => d,
        None => panic!("Unable to connect to a wayland compositor.")
    };

    // Use wayland_env! macro to get the globals and an event iterator
    let (mut env, _evt_iter) = WaylandEnv::init(display);

    // Get shortcuts to the globals.
    // Here we only use the version 1 of the interface, so no checks are needed.
    let compositor = env.compositor.as_ref().map(|o| &o.0).unwrap();
    let shell = env.shell.as_ref().map(|o| &o.0).unwrap();
    let shm = env.shm.as_ref().map(|o| &o.0).unwrap();

    let surface = compositor.create_surface();
    let shell_surface = shell.get_shell_surface(&surface);

    // create a tempfile to write on
    let mut tmp = tempfile::tempfile().ok().expect("Unable to create a tempfile.");
    // write the contents to it, lets put everything in dark red
    for _ in 0..10_000 {
        let _ = tmp.write_u32::<NativeEndian>(0xFF880000);
    }
    let _ = tmp.flush();

    let pool = shm.create_pool(tmp.as_raw_fd(), 40_000);
    // match a buffer on the part we wrote on
    let buffer = pool.create_buffer(0, 100, 100, 400, WlShmFormat::Argb8888 as u32);

    // make our surface as a toplevel one
    shell_surface.set_toplevel();
    // attach the buffer to it
    surface.attach(Some(&buffer), 0, 0);
    // commit
    surface.commit();

    env.display.sync_roundtrip().unwrap();

    loop {}
    
}
