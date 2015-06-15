extern crate byteorder;
extern crate wayland_client as wayland;

use byteorder::{WriteBytesExt, NativeEndian};

use std::fs::OpenOptions;
use std::io::Write;

use wayland::core::default_display;
use wayland::core::shm::ShmFormat;

fn main() {
    let display = default_display().expect("Unable to connect to Wayland server.");

    let registry = display.get_registry();
    display.sync_roundtrip();

    let compositor = registry.get_compositor().expect("Unable to get the compositor.");

    // first, create the shell surface
    let shell = registry.get_shell().expect("Unable to get the shell.");
    let shell_surface = shell.get_shell_surface(compositor.create_surface());

    // the the child surface
    let subcompositor = registry.get_subcompositor().expect("Unable to get the subcompositor.");
    let child_surface = subcompositor.get_subsurface(compositor.create_surface(), &shell_surface);

    // then obtain a buffer to store contents
    let shm = registry.get_shm().expect("Unable to get the shm.");
    // Not a good way to create a shared buffer, but this will do for this example.
    let mut tmp = OpenOptions::new().read(true).write(true).create(true).truncate(true)
                            .open("shm.tmp").ok().expect("Unable to create a tempfile.");
    // write the contents to it, lets put the main window in dark red
    for _ in 0..10_000 {
        let _ = tmp.write_u32::<NativeEndian>(0xFF880000);
    }
    // also prepare a child_surface in dark green
    for _ in 0..100 {
        let _ = tmp.write_u32::<NativeEndian>(0xFF008800);
    }
    let _ = tmp.flush();
    // create a shm_pool from this tempfile
    let pool = shm.pool_from_fd(&tmp, 40_400);
    // match a buffer on the part we wrote on
    let buffer_parent = pool.create_buffer(0, 100, 100, 400, ShmFormat::ARGB8888)
                            .expect("Could not create buffer.");
    let buffer_child = pool.create_buffer(40_000, 10, 10, 40, ShmFormat::ARGB8888)
                           .expect("Could not create buffer.");

    // prepare child surface
    child_surface.attach(&buffer_child, 0, 0);
    child_surface.set_position(45, 45);
    child_surface.set_sync(true);
    child_surface.commit();

    // prepare parent surface
    shell_surface.set_toplevel();
    shell_surface.attach(&buffer_parent, 0, 0);
    shell_surface.commit();

    display.sync_roundtrip();

    loop {}
}