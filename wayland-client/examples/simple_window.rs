extern crate byteorder;
extern crate tempfile;
#[macro_use(event_enum)]
extern crate wayland_client;

use std::cmp::min;
use std::io::Write;
use std::os::unix::io::AsRawFd;

use byteorder::{NativeEndian, WriteBytesExt};

use wayland_client::protocol::{wl_compositor, wl_keyboard, wl_pointer, wl_seat, wl_shell, wl_shm};
use wayland_client::sinks::blocking_message_iterator;
use wayland_client::{Display, GlobalManager};

// declare an event enum containing the events we want to receive in the iterator
event_enum!(
    Events |
    Pointer => wl_pointer::WlPointer,
    Keyboard => wl_keyboard::WlKeyboard
);

fn main() {
    let (display, mut event_queue) = Display::connect_to_env().unwrap();
    let globals = GlobalManager::new(&display);

    // roundtrip to retrieve the globals list
    event_queue.sync_roundtrip().unwrap();

    /*
     * Create a buffer with window contents
     */

    // buffer (and window) width and height
    let buf_x: u32 = 320;
    let buf_y: u32 = 240;

    // create a tempfile to write the contents of the window on
    let mut tmp = tempfile::tempfile().ok().expect("Unable to create a tempfile.");
    // write the contents to it, lets put a nice color gradient
    for i in 0..(buf_x * buf_y) {
        let x = (i % buf_x) as u32;
        let y = (i / buf_x) as u32;
        let r: u32 = min(((buf_x - x) * 0xFF) / buf_x, ((buf_y - y) * 0xFF) / buf_y);
        let g: u32 = min((x * 0xFF) / buf_x, ((buf_y - y) * 0xFF) / buf_y);
        let b: u32 = min(((buf_x - x) * 0xFF) / buf_x, (y * 0xFF) / buf_y);
        let _ = tmp.write_u32::<NativeEndian>((0xFF << 24) + (r << 16) + (g << 8) + b);
    }
    let _ = tmp.flush();

    /*
     * Init wayland objects
     */

    // The compositor allows us to creates surfaces
    let compositor = globals
        .instantiate_exact::<wl_compositor::WlCompositor, _>(1, |comp| comp.implement_dummy())
        .unwrap();
    let surface = compositor
        .create_surface(|surface| surface.implement_dummy())
        .unwrap();

    // The SHM allows us to share memory with the server, and create buffers
    // on this shared memory to paint our surfaces
    let shm = globals
        .instantiate_exact::<wl_shm::WlShm, _>(1, |shm| shm.implement_dummy())
        .unwrap();
    let pool = shm
        .create_pool(
            tmp.as_raw_fd(),            // RawFd to the tempfile serving as shared memory
            (buf_x * buf_y * 4) as i32, // size in bytes of the shared memory (4 bytes per pixel)
            |pool| pool.implement_dummy(),
        )
        .unwrap();
    let buffer = pool
        .create_buffer(
            0,                        // Start of the buffer in the pool
            buf_x as i32,             // width of the buffer in pixels
            buf_y as i32,             // height of the buffer in pixels
            (buf_x * 4) as i32,       // number of bytes between the beginning of two consecutive lines
            wl_shm::Format::Argb8888, // chosen encoding for the data
            |buffer| buffer.implement_dummy(),
        )
        .unwrap();

    // The shell allows us to define our surface as a "toplevel", meaning the
    // server will treat it as a window
    //
    // NOTE: the wl_shell interface is actually deprecated in favour of the xdg_shell
    // protocol, available in wayland-protocols. But this will do for this example.
    let shell = globals
        .instantiate_exact::<wl_shell::WlShell, _>(1, |shell| shell.implement_dummy())
        .unwrap();
    let shell_surface = shell
        .get_shell_surface(&surface, |shellsurface| {
            shellsurface.implement_closure(
                |event, shell_surface| {
                    use wayland_client::protocol::wl_shell_surface::Event;
                    // This ping/pong mechanism is used by the wayland server to detect
                    // unresponsive applications
                    if let Event::Ping { serial } = event {
                        shell_surface.pong(serial);
                    }
                },
                (),
            )
        })
        .unwrap();

    // Set our surface as toplevel and define its contents
    shell_surface.set_toplevel();
    surface.attach(Some(&buffer), 0, 0);
    surface.commit();

    // initialize a seat to retrieve pointer & keyboard events
    //
    // we will dump them into a message iterator for easier handling
    let (sink, msg_iter) = blocking_message_iterator(event_queue.get_token());
    // to be handled properly this should be more dynamic, as more
    // than one seat can exist (and they can be created and destroyed
    // dynamically), however most "traditional" setups have a single
    // seat, so we'll keep it simple here
    let mut pointer_created = false;
    let mut keyboard_created = false;
    globals.instantiate_exact::<wl_seat::WlSeat, _>(1, |seat| {
        seat.implement_closure(
            move |event, seat| {
                // The capabilities of a seat are known at runtime and we retrieve
                // them via an events. 3 capabilities exists: pointer, keyboard, and touch
                // we are only interested in pointer here
                use wayland_client::protocol::wl_pointer::Event as PointerEvent;
                use wayland_client::protocol::wl_seat::{Capability, Event as SeatEvent};

                if let SeatEvent::Capabilities { capabilities } = event {
                    if !pointer_created && capabilities.contains(Capability::Pointer) {
                        // create the pointer only once
                        pointer_created = true;
                        seat.get_pointer(|pointer| pointer.implement(sink.clone(), ()))
                            .unwrap();
                    }
                    if !keyboard_created && capabilities.contains(Capability::Keyboard) {
                        // create the keyboard only once
                        keyboard_created = false;
                        seat.get_keyboard(|keyboard| keyboard.implement(sink.clone(), ()))
                            .unwrap();
                    }
                }
            },
            (),
        )
    });

    // the main loop of our program
    //
    // the message iterator will block waiting for new events
    for msg in msg_iter {
        match msg {
            Events::Pointer { event, .. } => match event {
                wl_pointer::Event::Enter {
                    surface_x, surface_y, ..
                } => {
                    println!("Pointer entered at ({}, {}).", surface_x, surface_y);
                }
                wl_pointer::Event::Leave { .. } => {
                    println!("Pointer left.");
                }
                wl_pointer::Event::Motion {
                    surface_x, surface_y, ..
                } => {
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
        }
    }
}
