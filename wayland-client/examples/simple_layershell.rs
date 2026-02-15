use std::{fs::File, os::unix::io::AsFd};
use wayland_client::{
    delegate_noop,
    protocol::{
        wl_buffer, wl_compositor, wl_keyboard, wl_registry, wl_seat, wl_shm, wl_shm_pool,
        wl_surface,
    },
    Connection, Dispatch, Proxy, QueueHandle, WEnum,
};

use wayland_protocols_wlr::layer_shell::v1::client::{
    zwlr_layer_shell_v1::{self, Layer},
    zwlr_layer_surface_v1::{self, Anchor},
};

use wayland_protocols::xdg::shell::client::xdg_wm_base;

fn main() {
    let conn = Connection::connect_to_env().unwrap();

    let mut event_queue = conn.new_event_queue();
    let qhandle = event_queue.handle();

    let display = conn.display();
    display.get_registry(&qhandle, ());

    let mut state = State {
        running: true,
        base_surface: None,
        layer_shell: None,
        layer_surface: None,
        buffer: None,
        wm_base: None,
    };

    event_queue.blocking_dispatch(&mut state).unwrap();

    if state.layer_shell.is_some() && state.wm_base.is_some() {
        state.init_layer_surface(&qhandle);
    }

    while state.running {
        event_queue.blocking_dispatch(&mut state).unwrap();
    }
}

struct State {
    running: bool,
    base_surface: Option<wl_surface::WlSurface>,
    layer_shell: Option<zwlr_layer_shell_v1::ZwlrLayerShellV1>,
    layer_surface: Option<zwlr_layer_surface_v1::ZwlrLayerSurfaceV1>,
    buffer: Option<wl_buffer::WlBuffer>,
    wm_base: Option<xdg_wm_base::XdgWmBase>,
}

impl Dispatch<wl_registry::WlRegistry, ()> for State {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global { name, interface, version } = event {
            if interface == zwlr_layer_shell_v1::ZwlrLayerShellV1::interface().name {
                let wl_layer = registry.bind::<zwlr_layer_shell_v1::ZwlrLayerShellV1, _, _>(
                    name,
                    version,
                    qh,
                    (),
                );
                state.layer_shell = Some(wl_layer);
            } else if interface == wl_compositor::WlCompositor::interface().name {
                let compositor =
                    registry.bind::<wl_compositor::WlCompositor, _, _>(name, version, qh, ());
                let surface = compositor.create_surface(qh, ());
                state.base_surface = Some(surface);
            } else if interface == wl_shm::WlShm::interface().name {
                let shm = registry.bind::<wl_shm::WlShm, _, _>(name, version, qh, ());

                let (init_w, init_h) = (3200, 240);

                let mut file = tempfile::tempfile().unwrap();
                draw(&mut file, (init_w, init_h));
                let pool = shm.create_pool(file.as_fd(), (init_w * init_h * 4) as i32, qh, ());
                let buffer = pool.create_buffer(
                    0,
                    init_w as i32,
                    init_h as i32,
                    (init_w * 4) as i32,
                    wl_shm::Format::Argb8888,
                    qh,
                    (),
                );
                state.buffer = Some(buffer.clone());
            } else if interface == wl_seat::WlSeat::interface().name {
                registry.bind::<wl_seat::WlSeat, _, _>(name, version, qh, ());
            } else if interface == xdg_wm_base::XdgWmBase::interface().name {
                let wm_base = registry.bind::<xdg_wm_base::XdgWmBase, _, _>(name, 1, qh, ());
                state.wm_base = Some(wm_base);
            }
        }
    }
}

delegate_noop!(State: ignore wl_compositor::WlCompositor);
delegate_noop!(State: ignore wl_surface::WlSurface);
delegate_noop!(State: ignore wl_shm::WlShm);
delegate_noop!(State: ignore wl_shm_pool::WlShmPool);
delegate_noop!(State: ignore wl_buffer::WlBuffer);

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

impl State {
    fn init_layer_surface(&mut self, qh: &QueueHandle<State>) {
        let layer = self.layer_shell.as_ref().unwrap().get_layer_surface(
            self.base_surface.as_ref().unwrap(),
            None,
            Layer::Top,
            "precure".to_string(),
            qh,
            (),
        );
        layer.set_anchor(Anchor::Bottom | Anchor::Right | Anchor::Left);
        layer.set_keyboard_interactivity(zwlr_layer_surface_v1::KeyboardInteractivity::OnDemand);
        layer.set_size(0, 30);
        layer.set_exclusive_zone(30);
        self.base_surface.as_ref().unwrap().commit();

        self.layer_surface = Some(layer);
    }
}

impl Dispatch<xdg_wm_base::XdgWmBase, ()> for State {
    fn event(
        _: &mut Self,
        wm_base: &xdg_wm_base::XdgWmBase,
        event: xdg_wm_base::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let xdg_wm_base::Event::Ping { serial } = event {
            wm_base.pong(serial);
        }
    }
}

impl Dispatch<wl_seat::WlSeat, ()> for State {
    fn event(
        _: &mut Self,
        seat: &wl_seat::WlSeat,
        event: wl_seat::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_seat::Event::Capabilities { capabilities: WEnum::Value(capabilities) } = event {
            if capabilities.contains(wl_seat::Capability::Keyboard) {
                seat.get_keyboard(qh, ());
            }
        }
    }
}

impl Dispatch<wl_keyboard::WlKeyboard, ()> for State {
    fn event(
        state: &mut Self,
        _: &wl_keyboard::WlKeyboard,
        event: wl_keyboard::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wl_keyboard::Event::Key { key, .. } = event {
            println!("key it is {key}");
            if key == 1 {
                // ESC key
                state.running = false;
            }
        }
    }
}

impl Dispatch<zwlr_layer_shell_v1::ZwlrLayerShellV1, ()> for State {
    fn event(
        _state: &mut Self,
        _proxy: &zwlr_layer_shell_v1::ZwlrLayerShellV1,
        _event: <zwlr_layer_shell_v1::ZwlrLayerShellV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<zwlr_layer_surface_v1::ZwlrLayerSurfaceV1, ()> for State {
    fn event(
        state: &mut Self,
        surface: &zwlr_layer_surface_v1::ZwlrLayerSurfaceV1,
        event: <zwlr_layer_surface_v1::ZwlrLayerSurfaceV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        if let zwlr_layer_surface_v1::Event::Configure { serial, .. } = event {
            surface.ack_configure(serial);
            let surface = state.base_surface.as_ref().unwrap();
            if let Some(ref buffer) = state.buffer {
                surface.attach(Some(buffer), 0, 0);
                surface.commit();
            }
        }
    }
}
