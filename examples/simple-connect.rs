extern crate byteorder;
extern crate tempfile;
extern crate wayland_client as wayland;

use byteorder::{WriteBytesExt, NativeEndian};

use std::io::Write;

use wayland::core::default_display;
use wayland::core::ShmFormat;

fn main() {
    let display = default_display().expect("Unable to connect to Wayland server.");

    let registry = display.get_registry();
    display.sync_roundtrip();

    let compositor = registry.get_compositor().expect("Unable to get the compositor.");

    // first, create the shell surface
    let surface = compositor.create_surface();
    let shell = registry.get_shell().expect("Unable to get the shell.");
    let shell_surface = shell.get_shell_surface(surface);

    // then obtain a buffer to store contents
    let shm = registry.get_shm().expect("Unable to get the shm.");
    // create a tempfile
    let mut tmp = tempfile::TempFile::new().ok().expect("Unable to create a tempfile.");
    // write the contents to it, lets put everything in dark red
    for _ in 0..10_000 {
        let _ = tmp.write_u32::<NativeEndian>(0xFF880000);
    }
    let _ = tmp.flush();
    // create a shm_pool from this tempfile
    let pool = shm.pool_from_fd(&tmp, 40_000);
    // match a buffer on the part we wrote on
    let buffer = pool.create_buffer(0, 100, 100, 400, ShmFormat::WL_SHM_FORMAT_ARGB8888)
                     .expect("Could not create buffer.");

    // make our surface as a toplevel one
    shell_surface.set_toplevel();
    // attach the buffer to it
    shell_surface.attach(&buffer, 0, 0);
    // commit
    shell_surface.commit();

    display.sync_roundtrip();

    loop {}
}