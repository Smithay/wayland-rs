mod helpers;

use helpers::{roundtrip, wayc, ways, TestClient, TestServer};

use ways::protocol::wl_compositor::WlCompositor as ServerCompositor;

use wayc::protocol::wl_compositor;
use wayc::{Implementation, Proxy};

struct CompImpl1;

impl Implementation<Proxy<wl_compositor::WlCompositor>, wl_compositor::Event> for CompImpl1 {
    fn receive(&mut self, _: wl_compositor::Event, _: Proxy<wl_compositor::WlCompositor>) {}
}

struct CompImpl2;

impl Implementation<Proxy<wl_compositor::WlCompositor>, wl_compositor::Event> for CompImpl2 {
    fn receive(&mut self, _: wl_compositor::Event, _: Proxy<wl_compositor::WlCompositor>) {}
}

#[test]
fn proxy_equals() {
    let mut server = TestServer::new();
    let loop_token = server.event_loop.token();
    server
        .display
        .create_global::<ServerCompositor, _>(&loop_token, 1, |_, _| {});

    let mut client = TestClient::new(&server.socket_name);
    let manager = wayc::GlobalManager::new(&client.display);

    roundtrip(&mut client, &mut server).unwrap();

    let compositor1 = manager
        .instantiate_auto::<wl_compositor::WlCompositor, _>(|newp| newp.implement(CompImpl1))
        .unwrap();

    let compositor2 = manager
        .instantiate_auto::<wl_compositor::WlCompositor, _>(|newp| newp.implement(CompImpl1))
        .unwrap();

    let compositor3 = compositor1.clone();

    assert!(compositor1 == compositor3);
    assert!(compositor1 != compositor2);
    assert!(compositor2 != compositor3);
}

#[test]
fn proxy_user_data() {
    let mut server = TestServer::new();
    let loop_token = server.event_loop.token();
    server
        .display
        .create_global::<ServerCompositor, _>(&loop_token, 1, |_, _| {});

    let mut client = TestClient::new(&server.socket_name);
    let manager = wayc::GlobalManager::new(&client.display);

    roundtrip(&mut client, &mut server).unwrap();

    let compositor1 = manager
        .instantiate_auto::<wl_compositor::WlCompositor, _>(|newp| newp.implement(CompImpl1))
        .unwrap();

    compositor1.set_user_data(0xDEADBEEF as usize as *mut _);

    let compositor2 = manager
        .instantiate_auto::<wl_compositor::WlCompositor, _>(|newp| newp.implement(CompImpl1))
        .unwrap();

    compositor2.set_user_data(0xBADC0FFE as usize as *mut _);

    let compositor3 = compositor1.clone();

    assert!(compositor1.get_user_data() as usize == 0xDEADBEEF);
    assert!(compositor2.get_user_data() as usize == 0xBADC0FFE);
    assert!(compositor3.get_user_data() as usize == 0xDEADBEEF);
}

#[test]
fn proxy_is_implemented() {
    let mut server = TestServer::new();
    let loop_token = server.event_loop.token();
    server
        .display
        .create_global::<ServerCompositor, _>(&loop_token, 1, |_, _| {});

    let mut client = TestClient::new(&server.socket_name);
    let manager = wayc::GlobalManager::new(&client.display);

    roundtrip(&mut client, &mut server).unwrap();

    let compositor1 = manager
        .instantiate_auto::<wl_compositor::WlCompositor, _>(|newp| newp.implement(CompImpl1))
        .unwrap();

    let compositor2 = manager
        .instantiate_auto::<wl_compositor::WlCompositor, _>(|newp| newp.implement(CompImpl2))
        .unwrap();

    let compositor3 = compositor1.clone();

    assert!(compositor1.is_implemented_with::<CompImpl1>());
    assert!(!compositor1.is_implemented_with::<CompImpl2>());
    assert!(compositor2.is_implemented_with::<CompImpl2>());
    assert!(!compositor2.is_implemented_with::<CompImpl1>());
    assert!(compositor3.is_implemented_with::<CompImpl1>());
    assert!(!compositor3.is_implemented_with::<CompImpl2>());
}
