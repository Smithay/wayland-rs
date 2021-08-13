use std::{fs::File, os::unix::prelude::AsRawFd};

use wayland_client::{
    event_enum,
    protocol::{wl_buffer, wl_compositor, wl_keyboard, wl_registry, wl_seat, wl_shm, wl_surface},
    proxy_internals::ProxyData,
    quick_sink, Connection, ConnectionHandle, QueueHandle, WEnum,
};

use wayland_protocols::xdg_shell::client::{xdg_surface, xdg_toplevel, xdg_wm_base};

fn main() {
    let cx = Connection::connect_to_env().unwrap();

    let mut event_queue = cx.new_event_queue();
    let qhandle = event_queue.handle();

    let display = cx.handle().display();
    display
        .get_registry(
            &mut cx.handle(),
            Some(qhandle.sink::<wl_registry::WlRegistry, _>(registry_event_handler).data()),
        )
        .unwrap();

    let mut state = State {
        running: true,
        base_surface: None,
        buffer: None,
        wm_base: None,
        xdg_surface: None,
        configured: false,
    };

    println!("Starting the example window app, press <ESC> to quit.");

    while state.running {
        event_queue.blocking_dispatch(&mut state).unwrap();
    }
}

struct State {
    running: bool,
    base_surface: Option<wl_surface::WlSurface>,
    buffer: Option<wl_buffer::WlBuffer>,
    wm_base: Option<xdg_wm_base::XdgWmBase>,
    xdg_surface: Option<(xdg_surface::XdgSurface, xdg_toplevel::XdgToplevel)>,
    configured: bool,
}

fn registry_event_handler(
    cx: &mut ConnectionHandle<'_>,
    (registry, event): (wl_registry::WlRegistry, wl_registry::Event),
    state: &mut State,
    qhandle: &QueueHandle<State>,
) {
    if let wl_registry::Event::Global { name, interface, .. } = event {
        match &interface[..] {
            "wl_compositor" => {
                let compositor = registry
                    .bind::<wl_compositor::WlCompositor>(cx, name, 1, Some(ProxyData::ignore()))
                    .unwrap();
                let surface = compositor.create_surface(cx, Some(ProxyData::ignore())).unwrap();
                state.base_surface = Some(surface);

                if state.wm_base.is_some() && state.xdg_surface.is_none() {
                    init_xdg_surface(cx, state, qhandle);
                }
            }
            "wl_shm" => {
                let shm =
                    registry.bind::<wl_shm::WlShm>(cx, name, 1, Some(ProxyData::ignore())).unwrap();

                let (init_w, init_h) = (320, 240);

                let mut file = tempfile::tempfile().unwrap();
                draw(&mut file, (init_w, init_h));
                let pool = shm
                    .create_pool(
                        cx,
                        file.as_raw_fd(),
                        (init_w * init_h * 4) as i32,
                        Some(ProxyData::ignore()),
                    )
                    .unwrap();
                let buffer = pool
                    .create_buffer(
                        cx,
                        0,
                        init_w as i32,
                        init_h as i32,
                        (init_w * 4) as i32,
                        wl_shm::Format::Argb8888,
                        Some(ProxyData::ignore()),
                    )
                    .unwrap();
                state.buffer = Some(buffer.clone());

                if state.configured {
                    let surface = state.base_surface.as_ref().unwrap();
                    surface.attach(cx, Some(buffer), 0, 0);
                    surface.commit(cx);
                }
            }
            "wl_seat" => {
                registry
                    .bind::<wl_seat::WlSeat>(
                        cx,
                        name,
                        1,
                        Some(qhandle.sink::<SeatEvent, _>(seat_events_handler).data()),
                    )
                    .unwrap();
            }
            "xdg_wm_base" => {
                let wm_base = registry
                    .bind::<xdg_wm_base::XdgWmBase>(
                        cx,
                        name,
                        1,
                        Some(qhandle.sink::<ShellEvent, _>(shell_events_handler).data()),
                    )
                    .unwrap();
                state.wm_base = Some(wm_base);

                if state.base_surface.is_some() && state.xdg_surface.is_none() {
                    init_xdg_surface(cx, state, qhandle);
                }
            }
            _ => {}
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

            let color = (a << 24) + (r << 16) + (g << 8) + b;
            buf.write_all(&color.to_ne_bytes()).unwrap();
        }
    }
    buf.flush().unwrap();
}

fn init_xdg_surface(
    cx: &mut ConnectionHandle<'_>,
    state: &mut State,
    qhandle: &QueueHandle<State>,
) {
    let wm_base = state.wm_base.as_ref().unwrap();
    let base_surface = state.base_surface.as_ref().unwrap();

    let xdg_surface = wm_base.get_xdg_surface(cx, base_surface.clone(), None).unwrap();
    let toplevel = xdg_surface.get_toplevel(cx, None).unwrap();
    toplevel.set_title(cx, "A fantastic window!".into());

    base_surface.commit(cx);

    state.xdg_surface = Some((xdg_surface, toplevel));
}

fn shell_events_handler(
    cx: &mut ConnectionHandle<'_>,
    event: ShellEvent,
    state: &mut State,
    _qhandle: &QueueHandle<State>,
) {
    match event {
        ShellEvent::Base { object, event: xdg_wm_base::Event::Ping { serial } } => {
            object.pong(cx, serial);
        }
        ShellEvent::Surface { object, event: xdg_surface::Event::Configure { serial, .. } } => {
            object.ack_configure(cx, serial);
            state.configured = true;
            let surface = state.base_surface.as_ref().unwrap();
            if let Some(ref buffer) = state.buffer {
                surface.attach(cx, Some(buffer.clone()), 0, 0);
                surface.commit(cx);
            }
        }
        ShellEvent::Toplevel { event: xdg_toplevel::Event::Close {}, .. } => {
            state.running = false;
        }
        _ => {}
    }
}

event_enum! {
    enum ShellEvent {
        xdg_wm_base::XdgWmBase => Base,
        xdg_surface::XdgSurface => Surface,
        xdg_toplevel::XdgToplevel => Toplevel
    }
}

fn seat_events_handler(
    cx: &mut ConnectionHandle<'_>,
    event: SeatEvent,
    state: &mut State,
    _qhandle: &QueueHandle<State>,
) {
    match event {
        SeatEvent::Seat {
            object,
            event: wl_seat::Event::Capabilities { capabilities: WEnum::Value(capabilities) },
        } => {
            if capabilities.contains(wl_seat::Capability::Keyboard) {
                object.get_keyboard(cx, None).unwrap();
            }
        }
        SeatEvent::Kbd { event: wl_keyboard::Event::Key { key, .. }, .. } => {
            if key == 1 {
                // ESC key
                state.running = false;
            }
        }
        _ => {}
    }
}

event_enum! {
    enum SeatEvent {
        wl_seat::WlSeat => Seat,
        wl_keyboard::WlKeyboard => Kbd
    }
}
