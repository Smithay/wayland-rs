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
    server
        .display
        .create_global::<ServerCompositor, _>(1, ways::Filter::new(|_: (_, _), _, _| {}));

    let mut client = TestClient::new(&server.socket_name);
    let manager = wayc::GlobalManager::new(&client.display_proxy);

    roundtrip(&mut client, &mut server).unwrap();

    let compositor1 = manager
        .instantiate_exact::<wl_compositor::WlCompositor>(1)
        .unwrap();

    let compositor2 = manager
        .instantiate_exact::<wl_compositor::WlCompositor>(1)
        .unwrap();

    let compositor3 = compositor1.clone();

    assert!(compositor1 == compositor3);
    assert!(compositor1 != compositor2);
    assert!(compositor2 != compositor3);
}

#[test]
fn proxy_user_data() {
    let mut server = TestServer::new();
    server
        .display
        .create_global::<ServerCompositor, _>(1, ways::Filter::new(|_: (_, _), _, _| {}));

    let mut client = TestClient::new(&server.socket_name);
    let manager = wayc::GlobalManager::new(&client.display_proxy);

    roundtrip(&mut client, &mut server).unwrap();

    let compositor1 = manager
        .instantiate_exact::<wl_compositor::WlCompositor>(1)
        .unwrap();
    let compositor1 = compositor1.as_ref();
    compositor1.user_data().set(|| 0xDEADBEEFusize);

    let compositor2 = manager
        .instantiate_exact::<wl_compositor::WlCompositor>(1)
        .unwrap();
    let compositor2 = compositor2.as_ref();
    compositor2.user_data().set(|| 0xBADC0FFEusize);

    let compositor3 = compositor1.clone();

    assert!(compositor1.user_data().get::<usize>() == Some(&0xDEADBEEF));
    assert!(compositor2.user_data().get::<usize>() == Some(&0xBADC0FFE));
    assert!(compositor3.user_data().get::<usize>() == Some(&0xDEADBEEF));
    assert!(compositor1.user_data().get::<u32>() == None);
}

#[test]
fn proxy_user_data_wrong_thread() {
    let mut server = TestServer::new();
    server
        .display
        .create_global::<ServerCompositor, _>(1, ways::Filter::new(|_: (_, _), _, _| {}));

    let mut client = TestClient::new(&server.socket_name);
    let manager = wayc::GlobalManager::new(&client.display_proxy);

    roundtrip(&mut client, &mut server).unwrap();

    let compositor = manager
        .instantiate_exact::<wl_compositor::WlCompositor>(1)
        .unwrap();
    let compositor: Proxy<_> = (**compositor).clone().into();
    compositor.user_data().set(|| 0xDEADBEEFusize);

    // we can access on the right thread
    assert!(compositor.user_data().get::<usize>().is_some());

    // but not in a new one
    ::std::thread::spawn(move || {
        assert!(compositor.user_data().get::<usize>().is_none());
    })
    .join()
    .unwrap();
}

#[test]
fn proxy_wrapper() {
    let mut server = TestServer::new();
    server
        .display
        .create_global::<ServerCompositor, _>(1, ways::Filter::new(|_: (_, _), _, _| {}));

    let mut client = TestClient::new(&server.socket_name);

    let mut event_queue_2 = client.display.create_event_queue();
    let manager = wayc::GlobalManager::new(&(**client.display).clone().attach(event_queue_2.token()));

    roundtrip(&mut client, &mut server).unwrap();

    // event_queue_2 has not been dispatched
    assert!(manager.list().len() == 0);

    event_queue_2
        .dispatch_pending(&mut (), |_, _, _| unreachable!())
        .unwrap();

    assert!(manager.list().len() == 1);
}

#[test]
fn proxy_create_unattached() {
    let mut server = TestServer::new();
    server
        .display
        .create_global::<ServerCompositor, _>(1, ways::Filter::new(|_: (_, _), _, _| {}));

    let mut client = TestClient::new(&server.socket_name);

    let manager = wayc::GlobalManager::new(&client.display_proxy);

    roundtrip(&mut client, &mut server).unwrap();

    let compositor = manager
        .instantiate_exact::<wl_compositor::WlCompositor>(1)
        .unwrap();
    let compositor = (**compositor).clone();

    let ret = ::std::thread::spawn(move || {
        compositor.create_surface(); // should panic
    })
    .join();

    // the child thread should have panicked
    assert!(ret.is_err())
}

#[test]
fn proxy_create_attached() {
    let mut server = TestServer::new();
    server
        .display
        .create_global::<ServerCompositor, _>(1, ways::Filter::new(|_: (_, _), _, _| {}));

    let mut client = TestClient::new(&server.socket_name);

    let manager = wayc::GlobalManager::new(&client.display_proxy);

    roundtrip(&mut client, &mut server).unwrap();

    let compositor = manager
        .instantiate_exact::<wl_compositor::WlCompositor>(1)
        .unwrap();
    let compositor = (**compositor).clone();

    let display2 = client.display.clone();

    ::std::thread::spawn(move || {
        let evq2 = display2.create_event_queue();
        let compositor_wrapper = compositor.as_ref().clone().attach(evq2.token());
        compositor_wrapper.create_surface(); // should not panic
    })
    .join()
    .unwrap();
}

#[test]
fn dead_proxies() {
    let mut server = TestServer::new();
    server
        .display
        .create_global::<ServerOutput, _>(3, ways::Filter::new(|_: (_, _), _, _| {}));

    let mut client = TestClient::new(&server.socket_name);
    let manager = wayc::GlobalManager::new(&client.display_proxy);

    roundtrip(&mut client, &mut server).unwrap();

    let output = manager.instantiate_exact::<wl_output::WlOutput>(3).unwrap();

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
