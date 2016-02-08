use std::iter::Iterator;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// All possible wayland events.
///
/// This enum does a first sorting of event, with a variant for each
/// protocol extension activated in cargo features.
///
/// As the number of variant of this enum can change depending on which cargo features are
/// activated, it should *never* be matched exhaustively. It contains an hidden, never-used
/// variant to ensure it.
#[derive(Debug)]
pub enum Event {
    Wayland(::wayland::WaylandProtocolEvent),
    #[cfg(all(feature = "unstable-protocols", feature="wpu-xdg_shell"))]
    XdgShellUnstableV5(::xdg_shell::XdgShellUnstableV5ProtocolEvent),
    #[doc(hidden)]
    __DoNotMatchThis,
}

pub type EventFifo = ::crossbeam::sync::MsQueue<Event>;

pub struct EventIterator {
    fifo: Arc<(EventFifo, AtomicBool)>
}

impl EventIterator {
    pub fn new() -> EventIterator {
        EventIterator {
            fifo: Arc::new((::crossbeam::sync::MsQueue::new(),AtomicBool::new(true)))
        }
    }
}

impl Drop for EventIterator {
    fn drop(&mut self) {
        self.fifo.1.store(false, Ordering::SeqCst);
    }
}

impl Iterator for EventIterator {
    type Item = Event;
    fn next(&mut self) -> Option<Event> {
        self.fifo.0.pop()
    }
}

pub fn get_eventiter_internals(evt: &EventIterator) -> Arc<(EventFifo, AtomicBool)> {
    evt.fifo.clone()
}

pub fn eventiter_from_internals(arc: Arc<(EventFifo, AtomicBool)>) -> EventIterator {
    EventIterator { fifo: arc }
}