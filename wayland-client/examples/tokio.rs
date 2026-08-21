// A variation on the `simple_window` example that uses tokio to poll the Wayland
// socket asynchronously.
//
// This displays a white window and prints pointer events to the console.

use std::{io::Write, os::unix::io::AsFd};
use tokio::io::unix::AsyncFd;
use wayland_client::{
    Connection, Dispatch, NoopIgnore, QueueHandle,
    globals::{GlobalListHandler, registry_queue_init},
    protocol::{wl_buffer, wl_compositor, wl_pointer, wl_seat, wl_shm, wl_surface},
};

use wayland_protocols::xdg::shell::client::{xdg_surface, xdg_toplevel, xdg_wm_base};

struct GlobalData;

#[tokio::main]
async fn main() {
    let conn = Connection::connect_to_env().unwrap();

    let (globals, event_queue) = registry_queue_init(&conn).unwrap();
    let qh = event_queue.handle();

    let wm_base =
        globals.bind_singleton::<xdg_wm_base::XdgWmBase, _, _>(&qh, 1..=1, GlobalData).unwrap();
    let compositor = globals
        .bind_singleton::<wl_compositor::WlCompositor, _, _>(&qh, 1..=1, NoopIgnore)
        .unwrap();

    let base_surface = compositor.create_surface(&qh, NoopIgnore);
    let xdg_surface = wm_base.get_xdg_surface(&base_surface, &qh, GlobalData);
    let _xdg_toplevel = xdg_surface.get_toplevel(&qh, GlobalData);
    base_surface.commit();

    for global in globals.contents().clone_list() {
        if global.interface == "wl_seat" {
            globals
                .bind_specific::<wl_seat::WlSeat, _, _>(&qh, global.name, 1..=1, GlobalData)
                .unwrap();
        }
    }

    let shm = globals.bind_singleton::<wl_shm::WlShm, _, _>(&qh, 1..=1, NoopIgnore).unwrap();

    let (init_w, init_h) = (320, 240);

    let mut file = tempfile::tempfile().unwrap();
    // File buffer with white pixels
    for _ in 0..init_w * init_h * 4 {
        file.write_all(&[255]).unwrap();
    }
    let pool = shm.create_pool(file.as_fd(), init_w * init_h * 4, &qh, NoopIgnore);
    let buffer = pool.create_buffer(
        0,
        init_w,
        init_h,
        init_w * 4,
        wl_shm::Format::Argb8888,
        &qh,
        NoopIgnore,
    );

    conn.flush().unwrap();

    let mut state = State { running: true, base_surface, buffer, configured: false };

    let mut event_queue = AsyncFd::new(event_queue).unwrap();
    while state.running {
        // Prepare read before polling file descritor
        // (Needed for synchronization if multiple threads are reading)
        if let Some(read_events_guard) = event_queue.get_ref().prepare_read() {
            let mut read_ready_guard = event_queue.readable_mut().await.unwrap();
            read_events_guard.read().unwrap();
            // `ReadEventsGuard` has read fd until `WouldBlock`; clear ready state
            read_ready_guard.clear_ready();
        }
        // Dispatch events to event handlers
        event_queue.get_mut().dispatch_pending(&mut state).unwrap();
        conn.flush().unwrap();
    }
}

struct State {
    running: bool,
    base_surface: wl_surface::WlSurface,
    buffer: wl_buffer::WlBuffer,
    configured: bool,
}

impl GlobalListHandler for State {}

impl Dispatch<xdg_wm_base::XdgWmBase, State> for GlobalData {
    fn event(
        &self,
        _: &mut State,
        wm_base: &xdg_wm_base::XdgWmBase,
        event: xdg_wm_base::Event,
        _: &Connection,
        _: &QueueHandle<State>,
    ) {
        if let xdg_wm_base::Event::Ping { serial } = event {
            wm_base.pong(serial);
        }
    }
}

impl Dispatch<xdg_surface::XdgSurface, State> for GlobalData {
    fn event(
        &self,
        state: &mut State,
        xdg_surface: &xdg_surface::XdgSurface,
        event: xdg_surface::Event,
        _: &Connection,
        _: &QueueHandle<State>,
    ) {
        if let xdg_surface::Event::Configure { serial, .. } = event {
            xdg_surface.ack_configure(serial);
            state.base_surface.attach(Some(&state.buffer), 0, 0);
            state.base_surface.commit();
            state.configured = true;
        }
    }
}

impl Dispatch<xdg_toplevel::XdgToplevel, State> for GlobalData {
    fn event(
        &self,
        state: &mut State,
        _: &xdg_toplevel::XdgToplevel,
        event: xdg_toplevel::Event,
        _: &Connection,
        _: &QueueHandle<State>,
    ) {
        if let xdg_toplevel::Event::Close = event {
            state.running = false;
        }
    }
}

impl Dispatch<wl_seat::WlSeat, State> for GlobalData {
    fn event(
        &self,
        _: &mut State,
        seat: &wl_seat::WlSeat,
        event: wl_seat::Event,
        _: &Connection,
        qh: &QueueHandle<State>,
    ) {
        if let wl_seat::Event::Capabilities { capabilities } = event {
            if capabilities.contains(wl_seat::Capability::Pointer) {
                seat.get_pointer(qh, GlobalData);
            }
        }
    }
}

impl Dispatch<wl_pointer::WlPointer, State> for GlobalData {
    fn event(
        &self,
        _state: &mut State,
        _: &wl_pointer::WlPointer,
        event: wl_pointer::Event,
        _: &Connection,
        _: &QueueHandle<State>,
    ) {
        println!("{:?}", event);
    }
}
