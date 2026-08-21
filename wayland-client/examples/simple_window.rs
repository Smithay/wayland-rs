use std::{fs::File, os::unix::io::AsFd};

use wayland_client::{
    Connection, Dispatch, NoopIgnore, QueueHandle,
    globals::{Global, GlobalList, GlobalListHandler, registry_queue_init},
    protocol::{wl_buffer, wl_compositor, wl_keyboard, wl_seat, wl_shm, wl_surface},
};

use wayland_protocols::xdg::shell::client::{xdg_surface, xdg_toplevel, xdg_wm_base};

struct GlobalData;

fn main() {
    let conn = Connection::connect_to_env().unwrap();

    let (globals, mut event_queue) = registry_queue_init(&conn).unwrap();
    let qh = event_queue.handle();

    let wm_base =
        globals.bind_singleton::<xdg_wm_base::XdgWmBase, _, _>(&qh, 1..=1, GlobalData).unwrap();
    let compositor = globals
        .bind_singleton::<wl_compositor::WlCompositor, _, _>(&qh, 1..=1, NoopIgnore)
        .unwrap();

    let base_surface = compositor.create_surface(&qh, NoopIgnore);
    let xdg_surface = wm_base.get_xdg_surface(&base_surface, &qh, GlobalData);
    let xdg_toplevel = xdg_surface.get_toplevel(&qh, GlobalData);
    xdg_toplevel.set_title("A fantastic window!".into());
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
    draw(&mut file, (init_w, init_h));
    let pool = shm.create_pool(file.as_fd(), (init_w * init_h * 4) as i32, &qh, NoopIgnore);
    let buffer = pool.create_buffer(
        0,
        init_w as i32,
        init_h as i32,
        (init_w * 4) as i32,
        wl_shm::Format::Argb8888,
        &qh,
        NoopIgnore,
    );

    let mut state = State { running: true, base_surface, buffer, configured: false };

    println!("Starting the example window app, press <ESC> to quit.");

    while state.running {
        event_queue.blocking_dispatch(&mut state).unwrap();
    }
}

struct State {
    running: bool,
    base_surface: wl_surface::WlSurface,
    buffer: wl_buffer::WlBuffer,
    configured: bool,
}

impl GlobalListHandler for State {
    fn runtime_add_global(
        &mut self,
        globals: &GlobalList,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        global: &Global,
    ) {
        if global.interface == "wl_seat" {
            globals
                .bind_specific::<wl_seat::WlSeat, _, _>(qh, global.name, 1..=1, GlobalData)
                .unwrap();
        }
    }
}

fn draw(tmp: &mut File, (buf_x, buf_y): (u32, u32)) {
    use std::{cmp::min, io::Write};
    let mut buf = std::io::BufWriter::new(tmp);
    for y in 0..buf_y {
        for x in 0..buf_x {
            let a = 0xFF;
            let r = min(((buf_x - x) * 0xFF) / buf_x, ((buf_y - y) * 0xFF) / buf_y);
            let g = min((x * 0xFF) / buf_x, ((buf_y - y) * 0xFF) / buf_y);
            let b = min(((buf_x - x) * 0xFF) / buf_x, (y * 0xFF) / buf_y);
            buf.write_all(&[b as u8, g as u8, r as u8, a as u8]).unwrap();
        }
    }
    buf.flush().unwrap();
}

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
            if capabilities.contains(wl_seat::Capability::Keyboard) {
                seat.get_keyboard(qh, GlobalData);
            }
        }
    }
}

impl Dispatch<wl_keyboard::WlKeyboard, State> for GlobalData {
    fn event(
        &self,
        state: &mut State,
        _: &wl_keyboard::WlKeyboard,
        event: wl_keyboard::Event,
        _: &Connection,
        _: &QueueHandle<State>,
    ) {
        if let wl_keyboard::Event::Key { key, .. } = event {
            if key == 1 {
                // ESC key
                state.running = false;
            }
        }
    }
}
