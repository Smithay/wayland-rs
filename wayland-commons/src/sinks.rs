//! Message sinks
//!
//! This is a common implementation re-used by wayland-client and wayland-server. See
//! their respective documentation for their use.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::{Rc, Weak};

/// The sink end of an message iterator.
///
/// This sink can be cloned and provided as implementation for wayland objects
/// as long as `T: From<I::Event>` or `T: From<I::Request>` (depending on whether
/// you are client-side or server-side).
pub struct Sink<T> {
    queue: Weak<RefCell<VecDeque<T>>>,
}

impl<T> Sink<T> {
    /// Push a new message to the associated message iterator
    ///
    /// If the iterator was dropped (and is thus no longer capable of
    /// retrieving it), the message will be silently dropped instead.
    pub fn push<U: Into<T>>(&self, msg: U) {
        if let Some(queue) = self.queue.upgrade() {
            queue.borrow_mut().push_back(msg.into())
        }
    }
}

impl<T> Clone for Sink<T> {
    fn clone(&self) -> Sink<T> {
        Sink {
            queue: self.queue.clone(),
        }
    }
}

/// A message iterator
///
/// It yields the various messages that have been pushed to it from its associated
/// sinks, in a MPSC fashion.
///
/// It returning `None` via the `Iterator` trait only means that no message is
/// pending. It may start yielding new messages afterwards. It never blocks.
pub struct MsgIter<T> {
    queue: Rc<RefCell<VecDeque<T>>>,
}

impl<T> Iterator for MsgIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        self.queue.borrow_mut().pop_front()
    }
}

/// Create a new message iterator and an associated sink.
pub fn message_iterator<T>() -> (Sink<T>, MsgIter<T>) {
    let queue = Rc::new(RefCell::new(VecDeque::new()));
    (
        Sink {
            queue: Rc::downgrade(&queue),
        },
        MsgIter { queue },
    )
}
