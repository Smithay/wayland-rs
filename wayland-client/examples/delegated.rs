#![allow(clippy::single_match)]

use wayland_client::{
    protocol::{
        wl_compositor::{self, WlCompositor},
        wl_display::{self, WlDisplay},
        wl_registry::{self, WlRegistry},
    },
    Connection, Dispatch, Proxy, QueueHandle,
};

/// A demonstration of how delegateing can make implementing protocols an implementation detail
///
/// Users of this module only need to implement `RegistryHandler` trait on their state,
/// Implementation of `Dispatch` can remain an internal detail of the module.
///
/// In a way you can pretend that everything inside of this submodule is a library / different crate
mod delegated {
    use super::*;

    pub trait RegistryHandler: 'static {
        fn state(&mut self) -> &mut Registry;
        fn new_global(&mut self, name: u32, interface: &str, version: u32);
    }

    pub struct Registry {
        wl_registry: WlRegistry,
    }

    impl Registry {
        /// Create a [`WlRegistry`] object, and handle it's events internally
        /// It can use [`RegistryHandler`] trait to callback to your `D` state.
        pub fn new<D: RegistryHandler>(qh: &QueueHandle<D>, display: &WlDisplay) -> Self {
            // Let's construct a `WlRegistry` object that dispatches it's events to our
            // `Registry::event` rather than to `D`,
            // that way it can remain an implementation detail
            let data = qh.make_data::<WlRegistry, _, Self>(());
            let wl_registry =
                display.send_constructor(wl_display::Request::GetRegistry {}, data).unwrap();

            Self { wl_registry }
        }

        pub fn wl_registry(&self) -> WlRegistry {
            self.wl_registry.clone()
        }
    }

    impl<D: RegistryHandler> Dispatch<WlRegistry, (), D> for Registry {
        /// Called whenever an object created via `make_data<WlRegistry, _, Registry>`
        /// receives a server event
        fn event(
            state: &mut D,
            _: &wl_registry::WlRegistry,
            event: wl_registry::Event,
            _: &(),
            _: &Connection,
            _: &QueueHandle<D>,
        ) {
            let _state = state.state();

            if let wl_registry::Event::Global { name, interface, version } = event {
                // Let's callback the user of this abstraction, informing them about new global
                state.new_global(name, &interface, version);
            }
        }
    }
}

struct AppData {
    registry: delegated::Registry,
    qh: QueueHandle<Self>,
}

impl delegated::RegistryHandler for AppData {
    fn state(&mut self) -> &mut delegated::Registry {
        &mut self.registry
    }

    // Even tho we did not implement WlRegistry, `delegated::Registry` implemented it for us,
    // and will call this method whenever new globals appear
    fn new_global(&mut self, name: u32, interface: &str, version: u32) {
        println!("[{}] {} (v{})", name, interface, version);

        match interface {
            "wl_compositor" => {
                self.registry.wl_registry().bind(name, version, &self.qh, ());
            }
            _ => {}
        }
    }
}

impl Dispatch<WlCompositor, ()> for AppData {
    fn event(
        _state: &mut Self,
        _proxy: &WlCompositor,
        _event: wl_compositor::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
    }
}

fn main() {
    let conn = Connection::connect_to_env().unwrap();

    let display = conn.display();

    let mut event_queue = conn.new_event_queue::<AppData>();
    let qh = event_queue.handle();

    // Let's ask `delegated::Registry` to implement `WlRegistry` for us, only calling us back whenever
    // necessary via `RegistryHandler` trait
    let registry = delegated::Registry::new(&qh, &display);

    let mut app = AppData { registry, qh: qh.clone() };

    println!("Advertized globals:");
    event_queue.roundtrip(&mut app).unwrap();
}
