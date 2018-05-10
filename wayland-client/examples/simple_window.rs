extern crate byteorder;
extern crate tempfile;
extern crate wayland_client;

use std::cmp::min;
use std::io::Write;
use std::os::unix::io::AsRawFd;

use byteorder::{NativeEndian, WriteBytesExt};

use wayland_client::protocol::wl_compositor::RequestsTrait as CompositorRequests;
use wayland_client::protocol::wl_display::RequestsTrait as DisplayRequests;
use wayland_client::protocol::wl_shell::RequestsTrait as ShellRequests;
use wayland_client::protocol::wl_shell_surface::RequestsTrait as ShellSurfaceRequests;
use wayland_client::protocol::wl_shm::RequestsTrait as ShmRequests;
use wayland_client::protocol::wl_shm_pool::RequestsTrait as PoolRequests;
use wayland_client::protocol::wl_surface::RequestsTrait as SurfaceRequests;
use wayland_client::protocol::{wl_compositor, wl_seat, wl_shell, wl_shell_surface, wl_shm};
use wayland_client::{Display, GlobalManager, Proxy};

fn main() {
    let (display, mut event_queue) = Display::connect_to_env().unwrap();
    let globals = GlobalManager::new(display.get_registry().unwrap());

    // roundtrip to retrieve the globals list
    event_queue.sync_roundtrip().unwrap();

    /*
     * Create a buffer with window contents
     */

    // buffer (and window) width and height
    let buf_x: u32 = 320;
    let buf_y: u32 = 240;

    // create a tempfile to write the conents of the window on
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
        .instantiate_auto::<wl_compositor::WlCompositor>()
        .unwrap()
        .implement(|_, _| {});
    let surface = compositor.create_surface().unwrap().implement(|_, _| {});

    // The SHM allows us to share memory with the server, and create buffers
    // on this shared memory to paint our surfaces
    let shm = globals
        .instantiate_auto::<wl_shm::WlShm>()
        .unwrap()
        .implement(|_, _| {});
    let pool = shm.create_pool(
        tmp.as_raw_fd(),            // RawFd to the tempfile serving as shared memory
        (buf_x * buf_y * 4) as i32, // size in bytes of the shared memory (4 bytes per pixel)
    ).unwrap()
        .implement(|_, _| {});
    let buffer = pool.create_buffer(
        0,                        // Start of the buffer in the pool
        buf_x as i32,             // width of the buffer in pixels
        buf_y as i32,             // height of the buffer in pixels
        (buf_x * 4) as i32,       // number of bytes between the beginning of two consecutive lines
        wl_shm::Format::Argb8888, // chosen encoding for the data
    ).unwrap()
        .implement(|_, _| {});

    // The shell allows us to define our surface as a "toplevel", meaning the
    // server will treat it as a window
    //
    // NOTE: the wl_shell interface is actually deprecated in favour of the xdg_shell
    // protocol, available in wayland-protocols. But this will do for this example.
    let shell = globals
        .instantiate_auto::<wl_shell::WlShell>()
        .unwrap()
        .implement(|_, _| {});
    let shell_surface = shell.get_shell_surface(&surface).unwrap().implement(
        |event, shell_surface: Proxy<wl_shell_surface::WlShellSurface>| {
            use wayland_client::protocol::wl_shell_surface::{Event, RequestsTrait};
            // This ping/pong mechanism is used by the wayland server to detect
            // unresponsive applications
            if let Event::Ping { serial } = event {
                shell_surface.pong(serial);
            }
        },
    );

    // Set our surface as toplevel and define its contents
    shell_surface.set_toplevel();
    surface.attach(Some(&buffer), 0, 0);
    surface.commit();

    // initialize a seat to retrieve pointer events
    // to be handled properly this should be more dynamic, as more
    // than one seat can exist (and they can be created and destroyed
    // dynamically), however most "traditional" setups have a single
    // seat, so we'll keep it simple here
    let mut pointer_created = false;
    let _seat = globals.instantiate_auto::<wl_seat::WlSeat>().unwrap().implement(
        move |event, seat: Proxy<wl_seat::WlSeat>| {
            // The capabilities of a seat are known at runtime and we retrieve
            // them via an events. 3 capabilities exists: pointer, keyboard, and touch
            // we are only interested in pointer here
            use wayland_client::protocol::wl_pointer::Event as PointerEvent;
            use wayland_client::protocol::wl_seat::{Capability, Event as SeatEvent,
                                                    RequestsTrait as SeatRequests};

            if let SeatEvent::Capabilities { capabilities } = event {
                if !pointer_created && capabilities.contains(Capability::Pointer) {
                    // create the pointer only once
                    pointer_created = true;
                    seat.get_pointer().unwrap().implement(|event, _| match event {
                        PointerEvent::Enter {
                            surface_x, surface_y, ..
                        } => {
                            println!("Pointer entered at ({}, {}).", surface_x, surface_y);
                        }
                        PointerEvent::Leave { .. } => {
                            println!("Pointer left.");
                        }
                        PointerEvent::Motion {
                            surface_x, surface_y, ..
                        } => {
                            println!("Pointer moved to ({}, {}).", surface_x, surface_y);
                        }
                        PointerEvent::Button { button, state, .. } => {
                            println!("Button {} was {:?}.", button, state);
                        }
                        _ => {}
                    });
                }
            }
        },
    );

    loop {
        display.flush().unwrap();
        event_queue.dispatch().unwrap();
    }
}
