mod helpers;

use helpers::{roundtrip, wayc, ways, TestClient, TestServer};

use ways::protocol::wl_compositor::WlCompositor as ServerCompositor;
use ways::protocol::wl_output::WlOutput as ServerOutput;

use wayc::protocol::wl_compositor;
use wayc::protocol::wl_output;

#[test]
fn proxy_equals() {
    let mut server = TestServer::new();
    server.display.create_global::<ServerCompositor, _>(1, |_, _| {});

    let mut client = TestClient::new(&server.socket_name);
    let manager = wayc::GlobalManager::new(&client.display);

    roundtrip(&mut client, &mut server).unwrap();

    let compositor1 = manager
        .instantiate_auto::<wl_compositor::WlCompositor, _>(|newp| newp.implement(|_, _| {}, ()))
        .unwrap();

    let compositor2 = manager
        .instantiate_auto::<wl_compositor::WlCompositor, _>(|newp| newp.implement(|_, _| {}, ()))
        .unwrap();

    let compositor3 = compositor1.clone();

    assert!(compositor1 == compositor3);
    assert!(compositor1 != compositor2);
    assert!(compositor2 != compositor3);
}

#[test]
fn proxy_user_data() {
    let mut server = TestServer::new();
    server.display.create_global::<ServerCompositor, _>(1, |_, _| {});

    let mut client = TestClient::new(&server.socket_name);
    let manager = wayc::GlobalManager::new(&client.display);

    roundtrip(&mut client, &mut server).unwrap();

    let compositor1 = manager
        .instantiate_auto::<wl_compositor::WlCompositor, _>(|newp| newp.implement(|_, _| {}, 0xDEADBEEFusize))
        .unwrap();

    let compositor2 = manager
        .instantiate_auto::<wl_compositor::WlCompositor, _>(|newp| newp.implement(|_, _| {}, 0xBADC0FFEusize))
        .unwrap();

    let compositor3 = compositor1.clone();

    assert!(compositor1.user_data::<usize>() == Some(&0xDEADBEEF));
    assert!(compositor2.user_data::<usize>() == Some(&0xBADC0FFE));
    assert!(compositor3.user_data::<usize>() == Some(&0xDEADBEEF));
    assert!(compositor1.user_data::<u32>() == None);
}

#[test]
fn proxy_user_data_wrong_thread() {
    let mut server = TestServer::new();
    server.display.create_global::<ServerCompositor, _>(1, |_, _| {});

    let mut client = TestClient::new(&server.socket_name);
    let manager = wayc::GlobalManager::new(&client.display);

    roundtrip(&mut client, &mut server).unwrap();

    let compositor = manager
        .instantiate_auto::<wl_compositor::WlCompositor, _>(|newp| unsafe {
            newp.implement_nonsend(|_, _| {}, 0xDEADBEEFusize, &client.event_queue.get_token())
        })
        .unwrap();

    // we can access on the right thread
    assert!(compositor.user_data::<usize>().is_some());

    // but not in a new one
    ::std::thread::spawn(move || {
        assert!(compositor.user_data::<usize>().is_none());
    })
    .join()
    .unwrap();
}

#[test]
fn proxy_wrapper() {
    let mut server = TestServer::new();
    server.display.create_global::<ServerCompositor, _>(1, |_, _| {});

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
    server.display.create_global::<ServerOutput, _>(3, |_, _| {});

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
