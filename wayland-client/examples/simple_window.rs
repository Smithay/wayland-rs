// Allow single character names so clippy doesn't lint on x, y, r, g, b, which
// are reasonable variable names in this domain.
#![allow(clippy::many_single_char_names)]

use std::{
    cmp::min,
    io::{BufWriter, Write},
    os::unix::io::AsRawFd,
    process::exit,
    time::Instant,
};

use wayland_client::{
    event_enum,
    protocol::{wl_compositor, wl_keyboard, wl_pointer, wl_seat, wl_shm},
    Display, Filter, GlobalManager,
};
use wayland_protocols::xdg_shell::client::{xdg_surface, xdg_toplevel, xdg_wm_base};

// declare an event enum containing the events we want to receive in the iterator
event_enum!(
    Events |
    Pointer => wl_pointer::WlPointer,
    Keyboard => wl_keyboard::WlKeyboard
);

fn main() {
    let now = Instant::now();

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
    {
        let now = Instant::now();

        let mut buf = BufWriter::new(&mut tmp);
        for y in 0..buf_y {
            for x in 0..buf_x {
                let a = 0xFF;
                let r = min(((buf_x - x) * 0xFF) / buf_x, ((buf_y - y) * 0xFF) / buf_y);
                let g = min((x * 0xFF) / buf_x, ((buf_y - y) * 0xFF) / buf_y);
                let b = min(((buf_x - x) * 0xFF) / buf_x, (y * 0xFF) / buf_y);

                let color = (a << 24) + (r << 16) + (g << 8) + b;
                buf.write_all(&color.to_ne_bytes()).unwrap();
            }
        }
        buf.flush().unwrap();

        println!(
            "Time used to create the nice color gradient: {:?}",
            Instant::now().duration_since(now)
        );
    }

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

    let xdg_wm_base = globals
        .instantiate_exact::<xdg_wm_base::XdgWmBase>(2)
        .expect("Compositor does not support xdg_shell");

    xdg_wm_base.quick_assign(|xdg_wm_base, event, _| {
        if let xdg_wm_base::Event::Ping { serial } = event {
            xdg_wm_base.pong(serial);
        };
    });

    let xdg_surface = xdg_wm_base.get_xdg_surface(&surface);
    xdg_surface.quick_assign(move |xdg_surface, event, _| match event {
        xdg_surface::Event::Configure { serial } => {
            println!("xdg_surface (Configure)");
            xdg_surface.ack_configure(serial);
        }
        _ => unreachable!(),
    });

    let xdg_toplevel = xdg_surface.get_toplevel();
    xdg_toplevel.quick_assign(move |_, event, _| match event {
        xdg_toplevel::Event::Close => {
            exit(0);
        }
        xdg_toplevel::Event::Configure { width, height, states } => {
            println!(
                "xdg_toplevel (Configure) width: {}, height: {}, states: {:?}",
                width, height, states
            );
        }
        _ => unreachable!(),
    });
    xdg_toplevel.set_title("Simple Window".to_string());

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

    surface.commit();

    event_queue.sync_roundtrip(&mut (), |_, _, _| { /* we ignore unfiltered messages */ }).unwrap();

    surface.attach(Some(&buffer), 0, 0);
    surface.commit();

    println!("Time used before enter in the main loop: {:?}", Instant::now().duration_since(now));

    loop {
        event_queue.dispatch(&mut (), |_, _, _| { /* we ignore unfiltered messages */ }).unwrap();
    }
}
