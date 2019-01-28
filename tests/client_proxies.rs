mod helpers;

use helpers::{roundtrip, wayc, ways, TestClient, TestServer};

use ways::protocol::wl_compositor::WlCompositor as ServerCompositor;
use ways::protocol::wl_output::WlOutput as ServerOutput;

use wayc::protocol::wl_compositor;
use wayc::protocol::wl_output;
use wayc::Proxy;

#[test]
fn proxy_equals() {
    let mut server = TestServer::new();
    server.display.create_global::<ServerCompositor, _>(1, |_, _| {});

    let mut client = TestClient::new(&server.socket_name);
    let manager = wayc::GlobalManager::new(&client.display);

    roundtrip(&mut client, &mut server).unwrap();

    let compositor1 = manager
        .instantiate_auto::<wl_compositor::WlCompositor, _>(|newp| newp.implement_dummy())
        .unwrap();

    let compositor2 = manager
        .instantiate_auto::<wl_compositor::WlCompositor, _>(|newp| newp.implement_dummy())
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
        .instantiate_auto::<wl_compositor::WlCompositor, _>(|newp| {
            newp.implement_closure(|_, _| {}, 0xDEADBEEFusize)
        })
        .unwrap();
    let compositor1 = compositor1.as_ref();

    let compositor2 = manager
        .instantiate_auto::<wl_compositor::WlCompositor, _>(|newp| {
            newp.implement_closure(|_, _| {}, 0xBADC0FFEusize)
        })
        .unwrap();
    let compositor2 = compositor2.as_ref();

    let compositor3 = compositor1.clone();

    assert!(compositor1.user_data::<usize>() == Some(&0xDEADBEEF));
    assert!(compositor2.user_data::<usize>() == Some(&0xBADC0FFE));
    assert!(compositor3.user_data::<usize>() == Some(&0xDEADBEEF));
    assert!(compositor1.user_data::<u32>() == None);
}

#[cfg(not(features = "nothread"))]
#[test]
fn proxy_user_data_wrong_thread() {
    let mut server = TestServer::new();
    server.display.create_global::<ServerCompositor, _>(1, |_, _| {});

    let mut client = TestClient::new(&server.socket_name);
    let manager = wayc::GlobalManager::new(&client.display);

    roundtrip(&mut client, &mut server).unwrap();

    let compositor: Proxy<_> = manager
        .instantiate_auto::<wl_compositor::WlCompositor, _>(|newp| {
            newp.implement_closure(|_, _| {}, 0xDEADBEEFusize)
        })
        .unwrap()
        .into();

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
    let manager = wayc::GlobalManager::new(
        &(*client.display)
            .as_ref()
            .make_wrapper(&event_queue_2.get_token())
            .unwrap(),
    );

    roundtrip(&mut client, &mut server).unwrap();

    // event_queue_2 has not been dispatched
    assert!(manager.list().len() == 0);

    event_queue_2.dispatch_pending().unwrap();

    assert!(manager.list().len() == 1);
}

#[cfg(not(features = "nothread"))]
#[test]
fn proxy_implement_wrong_thread() {
    let mut server = TestServer::new();
    server.display.create_global::<ServerCompositor, _>(1, |_, _| {});

    let mut client = TestClient::new(&server.socket_name);

    let manager = wayc::GlobalManager::new(&client.display);

    roundtrip(&mut client, &mut server).unwrap();

    let compositor = manager
        .instantiate_auto::<wl_compositor::WlCompositor, _>(|newp| newp.implement_dummy())
        .unwrap();

    let ret = ::std::thread::spawn(move || {
        compositor
            .create_surface(|newp| newp.implement_closure(|_, _| {}, ())) // should panic
            .unwrap();
    })
    .join();

    // the child thread should have panicked
    assert!(ret.is_err())
}

#[cfg(not(features = "nothread"))]
#[test]
fn proxy_implement_wrapper_threaded() {
    let mut server = TestServer::new();
    server.display.create_global::<ServerCompositor, _>(1, |_, _| {});

    let mut client = TestClient::new(&server.socket_name);

    let manager = wayc::GlobalManager::new(&client.display);

    roundtrip(&mut client, &mut server).unwrap();

    let compositor = manager
        .instantiate_auto::<wl_compositor::WlCompositor, _>(|newp| newp.implement_dummy())
        .unwrap();

    let display2 = client.display.clone();

    ::std::thread::spawn(move || {
        let evq2 = display2.create_event_queue();
        let compositor_wrapper = compositor.as_ref().make_wrapper(&evq2.get_token()).unwrap();
        compositor_wrapper
            .create_surface(|newp| newp.implement_closure(|_, _| {}, ())) // should not panic
            .unwrap();
    })
    .join()
    .unwrap();
}

#[cfg(not(features = "nothread"))]
#[test]
fn proxy_implement_threadsafe_wrong_thread() {
    let mut server = TestServer::new();
    server.display.create_global::<ServerCompositor, _>(1, |_, _| {});

    let mut client = TestClient::new(&server.socket_name);

    let manager = wayc::GlobalManager::new(&client.display);

    roundtrip(&mut client, &mut server).unwrap();

    let compositor = manager
        .instantiate_auto::<wl_compositor::WlCompositor, _>(|newp| newp.implement_dummy())
        .unwrap();

    ::std::thread::spawn(move || {
        compositor
            .create_surface(|newp| newp.implement_closure_threadsafe(|_, _| {}, ())) // should not panic
            .unwrap();
    })
    .join()
    .unwrap();
}

#[test]
fn dead_proxies() {
    let mut server = TestServer::new();
    server.display.create_global::<ServerOutput, _>(3, |_, _| {});

    let mut client = TestClient::new(&server.socket_name);
    let manager = wayc::GlobalManager::new(&client.display);

    roundtrip(&mut client, &mut server).unwrap();

    let output = manager
        .instantiate_auto::<wl_output::WlOutput, _>(|newp| newp.implement_dummy())
        .unwrap();

    roundtrip(&mut client, &mut server).unwrap();

    let output2 = output.clone();

    assert!(output == output2);
    assert!(output.as_ref().is_alive());
    assert!(output2.as_ref().is_alive());

    // kill the output
    output.release();

    // dead proxies are never equal
    assert!(output != output2);
    assert!(!output.as_ref().is_alive());
    assert!(!output2.as_ref().is_alive());
}
