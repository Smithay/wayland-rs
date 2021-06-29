//! Filter

use std::{cell::RefCell, collections::VecDeque, rc::Rc};

/// Holder of global dispatch-related data
///
/// This struct serves as a dynamic container for the dispatch-time
/// global data that you gave to the dispatch method, and is given as
/// input to all your callbacks. It allows you to share global state
/// between your filters.
///
/// The main method of interest is the `get` method, which allows you to
/// access a `&mut _` reference to the global data itself. The other methods
/// are mostly used internally by the crate.
pub struct DispatchData<'a> {
    data: &'a mut dyn std::any::Any,
}

impl<'a> std::fmt::Debug for DispatchData<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DispatchData { ... }")
    }
}

impl<'a> DispatchData<'a> {
    /// Access the dispatch data knowing its type
    ///
    /// Will return `None` if the provided type is not the correct
    /// inner type.
    pub fn get<T: std::any::Any>(&mut self) -> Option<&mut T> {
        self.data.downcast_mut()
    }

    /// Wrap a mutable reference
    ///
    /// This creates a new `DispatchData` from a mutable reference
    pub fn wrap<T: std::any::Any>(data: &'a mut T) -> DispatchData<'a> {
        DispatchData { data }
    }

    /// Reborrows this `DispatchData` to create a new one with the same content
    ///
    /// This is a quick and cheap way to propagate the `DispatchData` down a
    /// callback stack by value. It is basically a noop only there to ease
    /// work with the borrow checker.
    pub fn reborrow(&mut self) -> DispatchData {
        DispatchData { data: &mut *self.data }
    }
}

struct Inner<E, F: ?Sized> {
    pending: RefCell<VecDeque<E>>,
    cb: RefCell<F>,
}

type DynInner<E> = Inner<E, dyn FnMut(E, &Filter<E>, DispatchData<'_>)>;

/// An event filter
///
/// Can be used in wayland-client and wayland-server to aggregate
/// messages from different objects into the same closure.
///
/// You need to provide it a closure of type `FnMut(E, &Filter<E>)`,
/// which will be called any time a message is sent to the filter
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

impl<E: std::fmt::Debug> std::fmt::Debug for Filter<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Filter").field("pending", &self.inner.pending).finish()
    }
}

impl<E> Clone for Filter<E> {
    fn clone(&self) -> Filter<E> {
        Filter { inner: self.inner.clone() }
    }
}

impl<E> Filter<E> {
    /// Create a new filter from given closure
    pub fn new<F: FnMut(E, &Filter<E>, DispatchData<'_>) + 'static>(f: F) -> Filter<E> {
        Filter {
            inner: Rc::new(Inner { pending: RefCell::new(VecDeque::new()), cb: RefCell::new(f) }),
        }
    }

    /// Send a message to this filter
    pub fn send(&self, evt: E, mut data: DispatchData) {
        // gracefully handle reentrancy
        if let Ok(mut guard) = self.inner.cb.try_borrow_mut() {
            (&mut *guard)(evt, self, data.reborrow());
            // process all events that might have been enqueued by the cb
            while let Some(evt) = self.inner.pending.borrow_mut().pop_front() {
                (&mut *guard)(evt, self, data.reborrow());
            }
        } else {
            self.inner.pending.borrow_mut().push_back(evt);
        }
    }
}
