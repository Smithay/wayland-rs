extern crate nix;
extern crate wayland_server as ways;

use std::cell::Cell;
use std::rc::Rc;

use ways::sources::*;
use ways::EventLoop;

#[test]
fn timer_wait() {
    let mut event_loop = EventLoop::new();

    let timer_fired = Rc::new(Cell::new(false));

    let mut timer = event_loop
        .token()
        .add_timer_event_source({
            let timer_signal = timer_fired.clone();
            move |_: TimerEvent, ()| {
                timer_signal.set(true);
            }
        })
        .map_err(|(e, _)| e)
        .unwrap();

    timer.set_delay_ms(1000); // 1s

    event_loop.dispatch(Some(100)).unwrap();

    // we waited only 100ms, the timer should not have fired yet
    assert!(!timer_fired.get());

    event_loop.dispatch(Some(2000)).unwrap();

    // we waited up to two seconds, the timer should have fired
    assert!(timer_fired.get());
}

#[test]
fn dispatch_idle() {
    let mut event_loop = EventLoop::new();

    let dispatched = Rc::new(Cell::new(false));

    let impl_dispatched = dispatched.clone();
    event_loop
        .token()
        .add_idle_event_source(move |_, _| impl_dispatched.set(true));

    event_loop.dispatch(Some(1)).unwrap();

    assert!(dispatched.get());
}

#[test]
fn event_loop_run() {
    let mut event_loop = EventLoop::new();
    let signal = event_loop.signal();

    let mut timer = event_loop
        .token()
        .add_timer_event_source(
            // stop loping when the timer fires
            move |_: TimerEvent, ()| signal.stop(),
        )
        .map_err(|(e, _)| e)
        .unwrap();

    timer.set_delay_ms(1000);

    event_loop.run().unwrap();
}
