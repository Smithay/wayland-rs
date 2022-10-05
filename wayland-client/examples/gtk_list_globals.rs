use gtk4::{gdk, glib, prelude::*};
use std::{collections::HashMap, future::poll_fn, os::unix::io::AsRawFd};
use wayland_client::{backend::Backend, protocol::wl_registry, Connection, Dispatch, QueueHandle};

struct State {
    list_box: gtk4::ListBox,
    rows: HashMap<u32, gtk4::Label>,
}

impl Dispatch<wl_registry::WlRegistry, ()> for State {
    fn event(
        state: &mut Self,
        _: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<State>,
    ) {
        match event {
            wl_registry::Event::Global { name, interface, version } => {
                // Add a row for the global to the `ListBox`
                let text = format!("[{}] {} (v{})", name, interface, version);
                let row = gtk4::Label::new(Some(&text));
                row.set_halign(gtk4::Align::Start);
                state.list_box.append(&row);
            }
            wl_registry::Event::GlobalRemove { name } => {
                // Remove the global's row from the `ListBox`
                let row = state.rows.remove(&name).unwrap();
                state.list_box.remove(&row);
            }
            _ => {}
        }
    }
}

fn main() {
    // Initialize GTK
    gtk4::init().unwrap();

    // Create a GTK window with an empty `ListBox`
    let list_box = gtk4::ListBox::new();
    let window = gtk4::Window::new();
    window.set_child(Some(&list_box));
    window.show();

    // Create a connection from the `GdkWaylandDisplay`
    let display =
        gdk::Display::default().unwrap().downcast::<gdk4_wayland::WaylandDisplay>().unwrap();
    let wl_display = display.wl_display().c_ptr();
    let connection =
        Connection::from_backend(unsafe { Backend::from_foreign_display(wl_display as _) });

    // Create an event queue and get registry GTK doesn't provide a way to get
    // registry events from its copy.
    let mut event_queue = connection.new_event_queue();
    let qh = event_queue.handle();
    let _registry = connection.display().get_registry(&qh, ());

    // Read from connection
    let fd = connection.prepare_read().unwrap().connection_fd().as_raw_fd();
    glib::source::unix_fd_add_local(fd, glib::IOCondition::IN, move |_, _| {
        connection.prepare_read().unwrap().read().unwrap();
        glib::Continue(true)
    });

    // Dispatch events when reads occur. Async version must be used since
    // GTK's types aren't thread safe.
    let mut state = State { list_box, rows: HashMap::new() };
    glib::MainContext::default().spawn_local(async move {
        poll_fn(|cx| event_queue.poll_dispatch_pending(cx, &mut state)).await.unwrap();
    });

    // Run GLib main loop
    glib::MainLoop::new(None, false).run();
}
