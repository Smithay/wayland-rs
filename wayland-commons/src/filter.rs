use std::{cell::RefCell, collections::VecDeque, rc::Rc};

struct Inner<E, F: ?Sized> {
    pending: RefCell<VecDeque<E>>,
    cb: RefCell<F>
}

pub struct Filter<E> {
    inner: Rc<Inner<E, dyn FnMut(E)>>
}

impl<E> Filter<E> {
    pub fn new<F: FnMut(E) + 'static>(f: F) -> Filter<E> {
        Filter {
            inner: Rc::new(Inner { pending: RefCell::new(VecDeque::new()), cb: RefCell::new(f) })
        }
    }

    pub fn send(&self, evt: E) {
        // gracefully handle reentrancy
        if let Ok(mut guard) = self.inner.cb.try_borrow_mut() {
            (&mut *guard)(evt);
            // process all events that might have been enqueued by the cb
            while let Some(evt) = self.inner.pending.borrow_mut().pop_front() {
                (&mut *guard)(evt);
            }
        } else {
            self.inner.pending.borrow_mut().push_back(evt);
        }
    }
}
