#[macro_use]
extern crate wayland_client;

extern crate tempfile;

extern crate byteorder;
use byteorder::{NativeEndian, WriteBytesExt};
use std::cmp::min;
use std::io::Write;
use std::os::unix::io::AsRawFd;
use wayland_client::EnvHandler;
use wayland_client::protocol::{wl_compositor, wl_pointer, wl_seat, wl_shell, wl_shell_surface, wl_shm};

wayland_env!(
    WaylandEnv,
    compositor: wl_compositor::WlCompositor,
    seat: wl_seat::WlSeat,
    shell: wl_shell::WlShell,
    shm: wl_shm::WlShm
);

fn shell_surface_impl() -> wl_shell_surface::Implementation<()> {
    wl_shell_surface::Implementation {
        ping: |_, _, shell_surface, serial| {
            shell_surface.pong(serial);
        },
        configure: |_, _, _, _, _, _| { /* not used in this example */ },
        popup_done: |_, _, _| { /* not used in this example */ },
    }
}

fn pointer_impl() -> wl_pointer::Implementation<()> {
    wl_pointer::Implementation {
        enter: |_, _, _pointer, _serial, _surface, x, y| {
            println!("Pointer entered surface at ({},{}).", x, y);
        },
        leave: |_, _, _pointer, _serial, _surface| {
            println!("Pointer left surface.");
        },
        motion: |_, _, _pointer, _time, x, y| {
            println!("Pointer moved to ({},{}).", x, y);
        },
        button: |_, _, _pointer, _serial, _time, button, state| {
            println!(
                "Button {} ({}) was {:?}.",
                match button {
                    272 => "Left",
                    273 => "Right",
                    274 => "Middle",
                    _ => "Unknown",
                },
                button,
                state
            );
        },
        axis: |_, _, _, _, _, _| { /* not used in this example */ },
        frame: |_, _, _| { /* not used in this example */ },
        axis_source: |_, _, _, _| { /* not used in this example */ },
        axis_discrete: |_, _, _, _, _| { /* not used in this example */ },
        axis_stop: |_, _, _, _, _| { /* not used in this example */ },
    }
}

fn main() {
    let (display, mut event_queue) = match wayland_client::default_connect() {
        Ok(ret) => ret,
        Err(e) => panic!("Cannot connect to wayland server: {:?}", e),
    };

    let registry = display.get_registry();

    let env_token = EnvHandler::<WaylandEnv>::init(&mut event_queue, &registry);

    event_queue.sync_roundtrip().unwrap();

    // buffer (and window) width and height
    let buf_x: u32 = 320;
    let buf_y: u32 = 240;

    // create a tempfile to write the conents of the window on
    let mut tmp = tempfile::tempfile()
        .ok()
        .expect("Unable to create a tempfile.");
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

    // retrieve the env
    let env = event_queue.state().get(&env_token).clone_inner().unwrap();

    // prepare the wayland surface
    let surface = env.compositor.create_surface();
    let shell_surface = env.shell.get_shell_surface(&surface);

    let pool = env.shm
        .create_pool(tmp.as_raw_fd(), (buf_x * buf_y * 4) as i32);
    // match a buffer on the part we wrote on
    let buffer = pool.create_buffer(
        0,
        buf_x as i32,
        buf_y as i32,
        (buf_x * 4) as i32,
        wl_shm::Format::Argb8888,
    ).expect("The pool cannot be already dead");

    // make our surface as a toplevel one
    shell_surface.set_toplevel();
    // attach the buffer to it
    surface.attach(Some(&buffer), 0, 0);
    // commit
    surface.commit();

    let pointer = env.seat
        .get_pointer()
        .expect("Seat cannot be already destroyed.");

    event_queue.register(&shell_surface, shell_surface_impl(), ());
    event_queue.register(&pointer, pointer_impl(), ());

    loop {
        display.flush().unwrap();
        event_queue.dispatch().unwrap();
    }
}
