// Allow single character names so clippy doesn't lint on x, y, r, g, b, which
// are reasonable variable names in this domain.
#![allow(clippy::many_single_char_names)]

use std::{cmp::min, io::Write, os::unix::io::AsRawFd};

use wayland_client::{
    event_enum,
    protocol::{wl_compositor, wl_keyboard, wl_pointer, wl_seat, wl_shell, wl_shm},
    Display, Filter, GlobalManager,
};

// declare an event enum containing the events we want to receive in the iterator
event_enum!(
    Events |
    Pointer => wl_pointer::WlPointer,
    Keyboard => wl_keyboard::WlKeyboard
);

fn main() {
    let display = Display::connect_to_env().unwrap();

    let mut event_queue = display.create_event_queue();

    let attached_display = (*display).clone().attach(event_queue.token());

    let globals = GlobalManager::new(&attached_display);

    // Make a synchronized roundtrip to the wayland server.
    //
    // When this returns it must be true that the server has already
    // sent us all available globals.
    event_queue.sync_roundtrip(&mut (), |_, _, _| unreachable!()).unwrap();

    /*
     * Create a buffer with window contents
     */

    // buffer (and window) width and height
    let buf_x: u32 = 320;
    let buf_y: u32 = 240;

    // create a tempfile to write the contents of the window on
    let mut tmp = tempfile::tempfile().expect("Unable to create a tempfile.");
    // write the contents to it, lets put a nice color gradient
    for i in 0..(buf_x * buf_y) {
        let x = i % buf_x;
        let y = i / buf_x;
        let a = 0xFF;
        let r = min(((buf_x - x) * 0xFF) / buf_x, ((buf_y - y) * 0xFF) / buf_y);
        let g = min((x * 0xFF) / buf_x, ((buf_y - y) * 0xFF) / buf_y);
        let b = min(((buf_x - x) * 0xFF) / buf_x, (y * 0xFF) / buf_y);
        tmp.write_all(&((a << 24) + (r << 16) + (g << 8) + b).to_ne_bytes()).unwrap();
    }
    let _ = tmp.flush();

    /*
     * Init wayland objects
     */

    // The compositor allows us to creates surfaces
    let compositor = globals.instantiate_exact::<wl_compositor::WlCompositor>(1).unwrap();
    let surface = compositor.create_surface();

    // The SHM allows us to share memory with the server, and create buffers
    // on this shared memory to paint our surfaces
    let shm = globals.instantiate_exact::<wl_shm::WlShm>(1).unwrap();
    let pool = shm.create_pool(
        tmp.as_raw_fd(),            // RawFd to the tempfile serving as shared memory
        (buf_x * buf_y * 4) as i32, // size in bytes of the shared memory (4 bytes per pixel)
    );
    let buffer = pool.create_buffer(
        0,                        // Start of the buffer in the pool
        buf_x as i32,             // width of the buffer in pixels
        buf_y as i32,             // height of the buffer in pixels
        (buf_x * 4) as i32,       // number of bytes between the beginning of two consecutive lines
        wl_shm::Format::Argb8888, // chosen encoding for the data
    );

    // The shell allows us to define our surface as a "toplevel", meaning the
    // server will treat it as a window
    //
    // NOTE: the wl_shell interface is actually deprecated in favour of the xdg_shell
    // protocol, available in wayland-protocols. But this will do for this example.
    let shell = globals
        .instantiate_exact::<wl_shell::WlShell>(1)
        .expect("Compositor does not support wl_shell");
    let shell_surface = shell.get_shell_surface(&surface);
    shell_surface.quick_assign(|shell_surface, event, _| {
        use wayland_client::protocol::wl_shell_surface::Event;
        // This ping/pong mechanism is used by the wayland server to detect
        // unresponsive applications
        if let Event::Ping { serial } = event {
            shell_surface.pong(serial);
        }
    });

    // Set our surface as toplevel and define its contents
    shell_surface.set_toplevel();
    surface.attach(Some(&buffer), 0, 0);
    surface.commit();

    // initialize a seat to retrieve pointer & keyboard events
    //
    // example of using a common filter to handle both pointer & keyboard events
    let common_filter = Filter::new(move |event, _, _| match event {
        Events::Pointer { event, .. } => match event {
            wl_pointer::Event::Enter { surface_x, surface_y, .. } => {
                println!("Pointer entered at ({}, {}).", surface_x, surface_y);
            }
            wl_pointer::Event::Leave { .. } => {
                println!("Pointer left.");
            }
            wl_pointer::Event::Motion { surface_x, surface_y, .. } => {
                println!("Pointer moved to ({}, {}).", surface_x, surface_y);
            }
            wl_pointer::Event::Button { button, state, .. } => {
                println!("Button {} was {:?}.", button, state);
            }
            _ => {}
        },
        Events::Keyboard { event, .. } => match event {
            wl_keyboard::Event::Enter { .. } => {
                println!("Gained keyboard focus.");
            }
            wl_keyboard::Event::Leave { .. } => {
                println!("Lost keyboard focus.");
            }
            wl_keyboard::Event::Key { key, state, .. } => {
                println!("Key with id {} was {:?}.", key, state);
            }
            _ => (),
        },
    });
    // to be handled properly this should be more dynamic, as more
    // than one seat can exist (and they can be created and destroyed
    // dynamically), however most "traditional" setups have a single
    // seat, so we'll keep it simple here
    let mut pointer_created = false;
    let mut keyboard_created = false;
    globals.instantiate_exact::<wl_seat::WlSeat>(1).unwrap().quick_assign(move |seat, event, _| {
        // The capabilities of a seat are known at runtime and we retrieve
        // them via an events. 3 capabilities exists: pointer, keyboard, and touch
        // we are only interested in pointer & keyboard here
        use wayland_client::protocol::wl_seat::{Capability, Event as SeatEvent};

        if let SeatEvent::Capabilities { capabilities } = event {
            if !pointer_created && capabilities.contains(Capability::Pointer) {
                // create the pointer only once
                pointer_created = true;
                seat.get_pointer().assign(common_filter.clone());
            }
            if !keyboard_created && capabilities.contains(Capability::Keyboard) {
                // create the keyboard only once
                keyboard_created = true;
                seat.get_keyboard().assign(common_filter.clone());
            }
        }
    });

    event_queue.sync_roundtrip(&mut (), |_, _, _| { /* we ignore unfiltered messages */ }).unwrap();

    loop {
        event_queue.dispatch(&mut (), |_, _, _| { /* we ignore unfiltered messages */ }).unwrap();
    }
}
