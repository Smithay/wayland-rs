extern crate byteorder;
extern crate tempfile;
extern crate wayland_client as wayland;

use byteorder::{WriteBytesExt, NativeEndian};

use std::io::Write;

use wayland::core::default_display;
use wayland::core::shm::ShmFormat;
use wayland::core::seat::KeyState;

fn main() {
    let display = default_display().expect("Unable to connect to Wayland server.");

    let registry = display.get_registry();
    display.sync_roundtrip();

    let compositor = registry.get_compositor().expect("Unable to get the compositor.");

    let seat = registry.get_seats().into_iter().next().expect("Unable to get the seat.");

    // first, create a simple surface like in simple-connect
    let surface = compositor.create_surface();
    let shell = registry.get_shell().expect("Unable to get the shell.");
    let mut shell_surface = shell.get_shell_surface(surface);
    let shm = registry.get_shm().expect("Unable to get the shm.");
    // create a tempfile as a shared buffer
    let mut tmp = tempfile::TempFile::new().ok().expect("Unable to create a tempfile.");
    for _ in 0..10_000 {
        let _ = tmp.write_u32::<NativeEndian>(0xFF880000);
    }
    let _ = tmp.flush();
    let pool = shm.pool_from_fd(&tmp, 40_000);
    let buffer = pool.create_buffer(0, 100, 100, 400, ShmFormat::ARGB8888)
                     .expect("Could not create buffer.");

    shell_surface.set_configure_callback(|_, w, h| {
        println!("Window was resized to {}x{}", w, h);
    });

    shell_surface.set_toplevel();
    shell_surface.attach(&buffer, 0, 0);
    shell_surface.commit();

    display.sync_roundtrip();

    // now, lets handle the pointer
    let mut pointer = seat.get_pointer().expect("Unable to get the pointer.");
    let my_surface_id = shell_surface.get_id();
    pointer.add_handled_surface(my_surface_id);
    pointer.set_enter_action(move |_, _, id, x, y| {
        if my_surface_id == id {
            println!("Pointer entered surface at ({},{}).", x, y);
        }
    });
    pointer.set_leave_action(move |_, _, id| {
        if my_surface_id == id {
            println!("Pointer left surface.");
        }
    });
    pointer.set_motion_action(move |_, _, x, y| {
        println!("Pointer moved to ({}, {}).", x, y);
    });
    pointer.set_button_action(move |_, _, _, b, s| {
        println!("Button {} is now in state {}.", b as u32, s as u32);
    });
    pointer.set_axis_action(move |_, _, _, a| {
        println!("Scrolled {}.", a);
    });

    let mut keyboard = seat.get_keyboard().expect("Unable to get the keyboard.");
    keyboard.set_key_action(move |_, _, _, key, status| {
        if status == KeyState::Released {
            println!("Key {} released.", key);
        } else {
            println!("Key {} pressed.", key);
        }
    });

    loop {
        let _ = display.flush();
        display.dispatch();
    }
}
