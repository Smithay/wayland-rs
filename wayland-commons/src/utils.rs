//! Various utilities used for other implementations

use std::any::Any;
use std::thread::{self, ThreadId};

/// A wrapper for user data, able to store any type, and correctly
/// handling access from a wrong thread
pub struct UserData {
    inner: UserDataInner,
}

enum UserDataInner {
    ThreadSafe(Box<Any + Send + Sync + 'static>),
    NonThreadSafe(Box<Any + 'static>, ThreadId),
    Empty,
}

// UserData itself is always threadsafe, as it only gives access to its
// content if it is send+sync or we are on the right thread
unsafe impl Send for UserData {}
unsafe impl Sync for UserData {}

impl UserData {
    /// Create a new `UserData` using a threadsafe type
    ///
    /// Its contents can be accessed from any thread.
    pub fn new_threadsafe<T: Send + Sync + 'static>(value: T) -> UserData {
        UserData {
            inner: UserDataInner::ThreadSafe(Box::new(value)),
        }
    }

    /// Create a new `UserData` using a non-threadsafe type
    ///
    /// Its contents can only be accessed from the same thread as the one you
    /// are creating it.
    pub fn new<T: 'static>(value: T) -> UserData {
        UserData {
            inner: UserDataInner::NonThreadSafe(Box::new(value), thread::current().id()),
        }
    }

    /// Create a new `UserData` containing nothing
    pub fn empty() -> UserData {
        UserData {
            inner: UserDataInner::Empty,
        }
    }

    /// Attempt to access the wrapped user data
    ///
    /// Will return `None` if either:
    ///
    /// - The requested type `T` does not match the itype used for construction
    /// - This `UserData` has been created using the non-threadsafe variant and access
    ///   is attempted from an other thread than the one it was created on
    pub fn get<T: 'static>(&self) -> Option<&T> {
        match self.inner {
            UserDataInner::ThreadSafe(ref val) => val.downcast_ref(),
            UserDataInner::NonThreadSafe(ref val, threadid) => {
                // only give access if we are on the right thread
                if threadid == thread::current().id() {
                    val.downcast_ref()
                } else {
                    None
                }
            }
            UserDataInner::Empty => None,
        }
    }
}
