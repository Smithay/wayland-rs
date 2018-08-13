mod helpers;

use helpers::{roundtrip, wayc, ways, TestClient, TestServer};

use ways::protocol::wl_compositor::WlCompositor as ServerCompositor;
use ways::protocol::wl_output::WlOutput as ServerOutput;

use wayc::protocol::wl_compositor;
use wayc::protocol::wl_output;
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
        .instantiate_auto::<wl_compositor::WlCompositor, _>(|newp| newp.implement(CompImpl1, ()))
        .unwrap();

    let compositor2 = manager
        .instantiate_auto::<wl_compositor::WlCompositor, _>(|newp| newp.implement(CompImpl1, ()))
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
        .instantiate_auto::<wl_compositor::WlCompositor, _>(|newp| newp.implement(CompImpl1, 0xDEADBEEFusize))
        .unwrap();

    let compositor2 = manager
        .instantiate_auto::<wl_compositor::WlCompositor, _>(|newp| newp.implement(CompImpl1, 0xBADC0FFEusize))
        .unwrap();

    let compositor3 = compositor1.clone();

    assert!(compositor1.user_data::<usize>() == Some(&0xDEADBEEF));
    assert!(compositor2.user_data::<usize>() == Some(&0xBADC0FFE));
    assert!(compositor3.user_data::<usize>() == Some(&0xDEADBEEF));
    assert!(compositor1.user_data::<u32>() == None);
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
        .instantiate_auto::<wl_compositor::WlCompositor, _>(|newp| newp.implement(CompImpl1, ()))
        .unwrap();

    let compositor2 = manager
        .instantiate_auto::<wl_compositor::WlCompositor, _>(|newp| newp.implement(CompImpl2, ()))
        .unwrap();

    let compositor3 = compositor1.clone();

    assert!(compositor1.is_implemented_with::<CompImpl1>());
    assert!(!compositor1.is_implemented_with::<CompImpl2>());
    assert!(compositor2.is_implemented_with::<CompImpl2>());
    assert!(!compositor2.is_implemented_with::<CompImpl1>());
    assert!(compositor3.is_implemented_with::<CompImpl1>());
    assert!(!compositor3.is_implemented_with::<CompImpl2>());
}

#[test]
fn proxy_wrapper() {
    let mut server = TestServer::new();
    let loop_token = server.event_loop.token();
    server
        .display
        .create_global::<ServerCompositor, _>(&loop_token, 1, |_, _| {});

    let mut client = TestClient::new(&server.socket_name);

    let mut event_queue_2 = client.display.create_event_queue();
    let manager = wayc::GlobalManager::new(&client.display.make_wrapper(&event_queue_2.get_token()).unwrap());

    roundtrip(&mut client, &mut server).unwrap();

    // event_queue_2 has not been dispatched
    assert!(manager.list().len() == 0);

    event_queue_2.dispatch_pending().unwrap();

    assert!(manager.list().len() == 1);
}

#[test]
fn dead_proxies() {
    use self::wl_output::RequestsTrait;

    let mut server = TestServer::new();
    let loop_token = server.event_loop.token();
    server
        .display
        .create_global::<ServerOutput, _>(&loop_token, 3, |_, _| {});

    let mut client = TestClient::new(&server.socket_name);
    let manager = wayc::GlobalManager::new(&client.display);

    roundtrip(&mut client, &mut server).unwrap();

    let output = manager
        .instantiate_auto::<wl_output::WlOutput, _>(|newp| newp.implement(|_, _| {}, ()))
        .unwrap();

    roundtrip(&mut client, &mut server).unwrap();

    let output2 = output.clone();

    assert!(output == output2);
    assert!(output.is_alive());
    assert!(output2.is_alive());

    // kill the output
    output.release();

    // dead proxies are never equal
    assert!(output != output2);
    assert!(!output.is_alive());
    assert!(!output2.is_alive());
}
