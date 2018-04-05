use std::cell::Cell;
use std::rc::Rc;

mod helpers;

use helpers::TestServer;

#[test]
fn dispatch_idle() {
    // Server setup
    //
    let mut server = TestServer::new();

    let dispatched = Rc::new(Cell::new(false));

    let impl_dispatched = dispatched.clone();
    server
        .event_loop
        .token()
        .add_idle_event_source(move |_, _| impl_dispatched.set(true));

    server.event_loop.dispatch(Some(1)).unwrap();

    assert!(dispatched.get());
}
