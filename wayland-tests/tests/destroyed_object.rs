use std::{
    os::fd::OwnedFd,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use wayland_client::Proxy;
use wayland_tests::{
    TestServer, globals, roundtrip, server_ignore_global_impl, server_ignore_impl, wayc, ways,
};

#[test]
fn destroyed_object_in_arg() {
    let mut server = TestServer::new();
    server
        .display
        .handle()
        .create_global::<ServerHandler, ways::protocol::wl_compositor::WlCompositor, _>(1, ());

    let (_, mut client) = server.add_client();
    let mut client_ddata = ClientHandler::default();
    let registry =
        client.display.get_registry(&client.event_queue.handle(), globals::GlobalListData);
    let qh = client.event_queue.handle();

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut ServerHandler).unwrap();

    let compositor = client_ddata
        .globals
        .bind::<wayc::protocol::wl_compositor::WlCompositor, _, _>(
            &qh,
            &registry,
            1..=1,
            wayc::Noop,
        )
        .unwrap();
    let surface = compositor.create_surface(&qh, wayc::NoopIgnore);
    let region = compositor.create_region(&qh, wayc::NoopIgnore);
    region.destroy();
    surface.set_input_region(Some(&region));

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut ServerHandler).unwrap();
}

#[test]
fn destroy_object_objectdata() {
    let mut server = TestServer::new();
    let (_, mut client) = server.add_client();

    let callback_data = Arc::new(CallbackData { destroyed: AtomicBool::new(false) });
    let _callback = client.display.send_constructor::<wayc::protocol::wl_callback::WlCallback>(
        wayc::protocol::wl_display::Request::Sync {},
        callback_data.clone(),
    );

    roundtrip(&mut client, &mut server, &mut (), &mut ()).unwrap();
    assert!(callback_data.destroyed.load(Ordering::Relaxed));
}

struct CallbackData {
    destroyed: AtomicBool,
}

impl wayc::backend::ObjectData for CallbackData {
    fn event(
        self: Arc<Self>,
        _: &wayc::backend::Backend,
        _: wayc::backend::protocol::Message<wayc::backend::ObjectId, OwnedFd>,
    ) -> Option<Arc<dyn wayc::backend::ObjectData + 'static>> {
        None
    }

    fn destroyed(&self, id: wayc::backend::ObjectId) {
        assert!(!self.destroyed.load(Ordering::Relaxed));
        assert!(!id.is_null());
        // `destroyed()` is called with object already marked as not alive, or it
        // could be invoked twice.
        #[cfg(feature = "client_system")]
        assert_eq!(id.as_ptr(), Err(wayc::backend::InvalidId));
        self.destroyed.store(true, Ordering::Relaxed);
    }
}

struct ServerHandler;

impl ways::Dispatch<ways::protocol::wl_compositor::WlCompositor, ServerHandler> for () {
    fn request(
        &self,
        _state: &mut ServerHandler,
        _: &ways::Client,
        _: &ways::protocol::wl_compositor::WlCompositor,
        request: ways::protocol::wl_compositor::Request,
        _: &ways::DisplayHandle,
        data_init: &mut ways::DataInit<'_, ServerHandler>,
    ) {
        match request {
            ways::protocol::wl_compositor::Request::CreateSurface { id } => {
                let _surface = data_init.init(id, ());
            }
            ways::protocol::wl_compositor::Request::CreateRegion { id } => {
                let _region = data_init.init(id, ());
            }
            ways::protocol::wl_compositor::Request::Release => {}
            _ => {
                unimplemented!()
            }
        }
    }
}

server_ignore_impl!(ServerHandler => [
    ways::protocol::wl_region::WlRegion,
    ways::protocol::wl_surface::WlSurface
]);
server_ignore_global_impl!(ServerHandler => [
    ways::protocol::wl_compositor::WlCompositor
]);

#[derive(Default)]
struct ClientHandler {
    globals: globals::GlobalList,
}

impl AsMut<globals::GlobalList> for ClientHandler {
    fn as_mut(&mut self) -> &mut globals::GlobalList {
        &mut self.globals
    }
}
