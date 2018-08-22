extern crate nix;
extern crate wayland_server as ways;

use std::cell::Cell;
use std::rc::Rc;

use ways::sources::*;
use ways::EventLoop;

use nix::sys::signal::{kill, sigprocmask, SigSet, SigmaskHow, Signal};
use nix::unistd::Pid;

// This test cannot run as a regular test because cargo would spawn a thread to run it,
// failing the sigprocmask...
fn main() {
    let mut event_loop = EventLoop::new();

    let signal_received = Rc::new(Cell::new(false));

    // block USR1
    let mut sigset = SigSet::empty();
    sigset.add(Signal::SIGUSR1);
    sigprocmask(SigmaskHow::SIG_BLOCK, Some(&sigset), None).unwrap();

    // add a signal event source for it
    event_loop
        .token()
        .add_signal_event_source(Signal::SIGUSR1, {
            let signal = signal_received.clone();
            move |SignalEvent(sig)| {
                assert!(sig == Signal::SIGUSR1);
                signal.set(true);
            }
        })
        .unwrap();

    // send ourselves a SIGUSR1
    kill(Pid::this(), Signal::SIGUSR1).unwrap();

    event_loop.dispatch(Some(10)).unwrap();

    assert!(signal_received.get());
}
