use std::iter::Iterator;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug)]
pub enum Event {
    Wayland(::wayland::WaylandProtocolEvent)
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