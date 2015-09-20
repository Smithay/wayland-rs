extern crate byteorder;
extern crate tempfile;
extern crate wayland_client as wayland;

use byteorder::{WriteBytesExt, NativeEndian};

use std::io::Write;
use std::os::unix::io::AsRawFd;

use wayland::{Proxy, EventIterator, Event};
use wayland::wayland::{WaylandProtocolEvent, WlRegistryEvent, WlRegistry};
use wayland::wayland::compositor::WlCompositor;
use wayland::wayland::shell::WlShell;
use wayland::wayland::shm::{WlShm, WlShmFormat};

fn main() {
    let mut display = match wayland::wayland::get_display() {
        Some(d) => d,
        None => panic!("Unable to connect to a wayland compositor.")
    };

    // Create an event iterator and assign it to the display
    // so that it is automatically inherited by all created objects
    let mut evt_iter = EventIterator::new();
    display.set_evt_iterator(&evt_iter);

    // Get the registry, to generate the events advertizing global objects
    let registry = display.get_registry();

    // Roundtrip, to make sure all event are dispatched to us
    display.sync_roundtrip();

    let (compositor, shell, shm) = fetch_globals(&mut evt_iter, &registry);

    let surface = compositor.create_surface();
    let shell_surface = shell.get_shell_surface(&surface);

    // create a tempfile to write on
    let mut tmp = tempfile::TempFile::new().ok().expect("Unable to create a tempfile.");
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

    display.sync_roundtrip();

    loop {}
    
}

fn fetch_globals(evt_iter: &mut EventIterator, rgt: &WlRegistry) -> (WlCompositor, WlShell, WlShm) {
    let mut compositor = None;
    let mut shell = None;
    let mut shm = None;
    for evt in evt_iter {
        match evt {
            // Global advertising events are `WlRegistryEvent::Global`
            Event::Wayland(WaylandProtocolEvent::WlRegistry(
                _, WlRegistryEvent::Global(name, interface, version)
            )) => {
                if interface == "wl_compositor" {
                    if version < WlCompositor::version() {
                        panic!("Compositor is too old to support wl_compositor version {}",
                            WlCompositor::version())
                    }
                    compositor = Some(unsafe { rgt.bind::<WlCompositor>(name) });
                } else if interface == "wl_shell" {
                    if version < WlShell::version() {
                        panic!("Compositor is too old to support wl_compositor version {}",
                            WlShell::version())
                    }
                    shell = Some(unsafe { rgt.bind::<WlShell>(name) });
                }
                else if interface == "wl_shm" {
                    if version < WlShm::version() {
                        panic!("Compositor is too old to support wl_compositor version {}",
                            WlShm::version())
                    }
                    shm = Some(unsafe { rgt.bind::<WlShm>(name) });
                }
            },
            // ignore everything else
            _ => {}
        }
    }
    match (compositor, shell, shm) {
        (Some(c), Some(se), Some(sh)) => (c, se, sh),
        (None, _, _) => panic!("Compositor did not advertize a wl_compositor."),
        (_, None, _) => panic!("Compositor did not advertize a wl_shell."),
        (_, _, None) => panic!("Compositor did not advertize a wl_shm."),
    }
}