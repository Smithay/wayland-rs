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

// This test cannot run as-is because cargo test spawns a new thread to run the
// test, failing the sigprocmask...
#[test]
#[ignore]
fn signal_event() {
    use nix::sys::signal::{kill, sigprocmask, SigSet, SigmaskHow, Signal};
    use nix::unistd::Pid;

    let mut event_loop = EventLoop::new();

    let signal_received = Rc::new(Cell::new(false));

    // block USR1
    let mut sigset = SigSet::empty();
    sigset.add(Signal::SIGUSR1);
    sigprocmask(SigmaskHow::SIG_BLOCK, Some(&sigset), None);

    // add a signal event source for it
    let signal_source = event_loop
        .token()
        .add_signal_event_source(Signal::SIGUSR1, {
            let signal = signal_received.clone();
            move |SignalEvent(sig), ()| {
                assert!(sig == Signal::SIGUSR1);
                signal.set(true);
            }
        })
        .map_err(|(e, _)| e)
        .unwrap();

    // send ourselves a SIGUSR1
    kill(Pid::this(), Signal::SIGUSR1);

    event_loop.dispatch(Some(10));

    assert!(signal_received.get());
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
