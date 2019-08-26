//! Message sinks
//!
//! The Wayland model naturally uses callbacks to handle the message you receive from the server.
//! However in some contexts, an iterator-based interface may be more practical to use, this is
//! what this module provides.
//!
//! The `message_iterator` function allows you to create a new message iterator. It is just a
//! regular MPSC, however its sending end (the `Sink<T>`) can be directly used as an implementation
//! for Wayland objects. This just requires that `T: From<(I::Event, I)>` (where `I` is the
//! interface of the object you're trying to implement). The `event_enum!` macro is provided to
//! easily generate an appropriate type joining events from different interfaces into a single
//! iterator.
//!
//! The `blocking_message_iterator` function is very similar, except the created message iterator
//! will be linked to an event queue, and will block on it rather than returning `None`, and is
//! thus able to drive an event loop.

use std::rc::Rc;

use wayland_commons::Interface;

use imp::EventQueueInner;
use {HandledBy, QueueToken};

pub use wayland_commons::sinks::{message_iterator, MsgIter, Sink};

impl<M, I> HandledBy<Sink<M>> for I
where
    I: Interface,
    M: From<(I::Event, I)>,
{
    fn handle(sink: &mut Sink<M>, event: I::Event, proxy: I) {
        sink.push((event, proxy));
    }
}

/// A message iterator linked to an event queue
///
/// Like a `MsgIter<T>`, but it is linked with an event queue, and
/// will `dispatch()` and block instead of returning `None` if no
/// events are pending.
pub struct BlockingMsgIter<T> {
    evt_queue: Rc<EventQueueInner>,
    iter: MsgIter<T>,
}

impl<T> Iterator for BlockingMsgIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        loop {
            match self.iter.next() {
                Some(msg) => return Some(msg),
                None => {
                    self.evt_queue
                        .dispatch()
                        .expect("Connection to the wayland server lost.");
                }
            }
        }
    }
}

/// Create a blokcing message iterator
///
/// Contrarily to `message_iterator`, this one will block on the event queue
/// represented by the provided token rather than returning `None` if no event is available.
pub fn blocking_message_iterator<T>(token: QueueToken) -> (Sink<T>, BlockingMsgIter<T>) {
    let (sink, iter) = message_iterator();
    (
        sink,
        BlockingMsgIter {
            iter,
            evt_queue: token.inner,
        },
    )
}