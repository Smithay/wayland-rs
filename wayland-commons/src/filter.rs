//! Filter

use std::{cell::RefCell, collections::VecDeque, rc::Rc};

struct Inner<E, F: ?Sized> {
    pending: RefCell<VecDeque<E>>,
    cb: RefCell<F>,
}

type DynInner<E> = Inner<E, dyn FnMut(E, &Filter<E>)>;

/// An event filter
///
/// Can be used in wayland-client and wayland-server to aggregate
/// messages from different objects into the same closure.
///
/// You need to provide it a closure of type `FnMut(E, &Filter<E>)`,
/// which will be called eny time a message is sent to the filter
/// via the `send(..)` method. Your closure also receives a handle
/// to the filter as argument, so that you can use it from within
/// the callback (to assign new wayland objects to this filter for
/// example).
///
/// The `Filter` can be cloned, and all clones send messages to the
/// same closure. However it is not threadsafe.
pub struct Filter<E> {
    inner: Rc<DynInner<E>>,
}

impl<E> Clone for Filter<E> {
    fn clone(&self) -> Filter<E> {
        Filter {
            inner: self.inner.clone(),
        }
    }
}

impl<E> Filter<E> {
    /// Create a new filter from given closure
    pub fn new<F: FnMut(E, &Filter<E>) + 'static>(f: F) -> Filter<E> {
        Filter {
            inner: Rc::new(Inner {
                pending: RefCell::new(VecDeque::new()),
                cb: RefCell::new(f),
            }),
        }
    }

    /// Send a message to this filter
    pub fn send(&self, evt: E) {
        // gracefully handle reentrancy
        if let Ok(mut guard) = self.inner.cb.try_borrow_mut() {
            (&mut *guard)(evt, self);
            // process all events that might have been enqueued by the cb
            while let Some(evt) = self.inner.pending.borrow_mut().pop_front() {
                (&mut *guard)(evt, self);
            }
        } else {
            self.inner.pending.borrow_mut().push_back(evt);
        }
    }
}
