//! Various utilities used for other implementations

use std::any::Any;
use std::thread::{self, ThreadId};

use self::list::AppendList;

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
    /// - The requested type `T` does not match the type used for construction
    /// - This `UserData` has been created using the non-threadsafe variant and access
    ///   is attempted from an other thread than the one it was created on
    pub fn get<T: 'static>(&self) -> Option<&T> {
        match self.inner {
            UserDataInner::ThreadSafe(ref val) => Any::downcast_ref::<T>(&**val),
            UserDataInner::NonThreadSafe(ref val, threadid) => {
                // only give access if we are on the right thread
                if threadid == thread::current().id() {
                    Any::downcast_ref::<T>(&**val)
                } else {
                    None
                }
            }
            UserDataInner::Empty => None,
        }
    }

    /// Attempt to mutably access the wrapped user data
    ///
    /// Will return `None` if either:
    ///
    /// - The requested type `T` does not match the type used for construction
    /// - This `UserData` has been created using the non-threadsafe variant and access
    ///   is attempted from an other thread than the one it was created on
    pub fn get_mut<T: 'static>(&mut self) -> Option<&mut T> {
        match self.inner {
            UserDataInner::ThreadSafe(ref mut val) => Any::downcast_mut::<T>(&mut **val),
            UserDataInner::NonThreadSafe(ref mut val, threadid) => {
                // only give access if we are on the right thread
                if threadid == thread::current().id() {
                    Any::downcast_mut::<T>(&mut **val)
                } else {
                    None
                }
            }
            UserDataInner::Empty => None,
        }
    }
}

/// A storage able to store several values of `UserData`
/// of different types. It behaves similarly to a `TypeMap`.
pub struct UserDataMap {
    list: AppendList<UserData>,
}

impl UserDataMap {
    /// Create a new map
    pub fn new() -> UserDataMap {
        UserDataMap {
            list: AppendList::new(),
        }
    }

    /// Attempt to access the wrapped user data of a given type
    ///
    /// Will return `None` if no value of type `T` is stored in this `UserDataMap`
    /// and accessible from this thread
    pub fn get<T: 'static>(&self) -> Option<&T> {
        for user_data in &self.list {
            if let Some(val) = user_data.get::<T>() {
                return Some(val);
            }
        }
        None
    }

    /// Attempt to mutably access the wrapped user data of a given type
    ///
    /// Will return `None` if no value of type `T` is stored in this `UserDataMap`
    /// and accessible from this thread
    pub fn get_mut<T: 'static>(&mut self) -> Option<&mut T> {
        for user_data in &mut self.list {
            if let Some(val) = user_data.get_mut::<T>() {
                return Some(val);
            }
        }
        None
    }

    /// Insert a value in the map if it is not already there
    ///
    /// This is the non-threadsafe variant, the type you insert don't have to be
    /// threadsafe, but they will not be visible from other threads (even if they are
    /// actually threadsafe).
    ///
    /// If the value does not already exists, the closure is called to create it and
    /// this function returns `true`. If the value already exists, the closure is not
    /// called, and this function returns `false`.
    pub fn insert_if_missing<T: 'static, F: FnOnce() -> T>(&self, init: F) -> bool {
        if self.get::<T>().is_some() {
            return false;
        }
        self.list.append(UserData::new(init()));
        true
    }

    /// Insert a value in the map if it is not already there
    ///
    /// This is the threadsafe variant, the type you insert must be threadsafe and will
    /// be visible from all threads.
    ///
    /// If the value does not already exists, the closure is called to create it and
    /// this function returns `true`. If the value already exists, the closure is not
    /// called, and this function returns `false`.
    pub fn insert_if_missing_threadsafe<T: Send + Sync + 'static, F: FnOnce() -> T>(&self, init: F) -> bool {
        if self.get::<T>().is_some() {
            return false;
        }
        self.list.append(UserData::new_threadsafe(init()));
        true
    }
}

impl Default for UserDataMap {
    fn default() -> UserDataMap {
        UserDataMap::new()
    }
}

mod list {
    /*
     * This is a lock-free append-only list, it is used as an implementation
     * detail of the UserDataMap.
     *
     * It was extracted from https://github.com/Diggsey/lockless under MIT license
     * Copyright © Diggory Blake <diggsey@googlemail.com>
     */

    use std::sync::atomic::{AtomicPtr, Ordering};
    use std::{mem, ptr};

    type NodePtr<T> = Option<Box<Node<T>>>;

    #[derive(Debug)]
    struct Node<T> {
        value: T,
        next: AppendList<T>,
    }

    #[derive(Debug)]
    pub struct AppendList<T>(AtomicPtr<Node<T>>);

    impl<T> AppendList<T> {
        fn node_into_raw(ptr: NodePtr<T>) -> *mut Node<T> {
            match ptr {
                Some(b) => Box::into_raw(b),
                None => ptr::null_mut(),
            }
        }
        unsafe fn node_from_raw(ptr: *mut Node<T>) -> NodePtr<T> {
            if ptr.is_null() {
                None
            } else {
                Some(Box::from_raw(ptr))
            }
        }

        fn new_internal(ptr: NodePtr<T>) -> Self {
            AppendList(AtomicPtr::new(Self::node_into_raw(ptr)))
        }

        pub fn new() -> Self {
            Self::new_internal(None)
        }

        pub fn append(&self, value: T) {
            self.append_list(AppendList::new_internal(Some(Box::new(Node {
                value,
                next: AppendList::new(),
            }))));
        }

        unsafe fn append_ptr(&self, p: *mut Node<T>) {
            loop {
                match self
                    .0
                    .compare_exchange_weak(ptr::null_mut(), p, Ordering::AcqRel, Ordering::Acquire)
                {
                    Ok(_) => return,
                    Err(head) => {
                        if !head.is_null() {
                            return (*head).next.append_ptr(p);
                        }
                    }
                }
            }
        }

        pub fn append_list(&self, other: AppendList<T>) {
            let p = other.0.load(Ordering::Acquire);
            mem::forget(other);
            unsafe { self.append_ptr(p) };
        }

        pub fn iter(&self) -> AppendListIterator<T> {
            AppendListIterator(&self.0)
        }

        pub fn iter_mut(&mut self) -> AppendListMutIterator<T> {
            AppendListMutIterator(&mut self.0)
        }
    }

    impl<'a, T> IntoIterator for &'a AppendList<T> {
        type Item = &'a T;
        type IntoIter = AppendListIterator<'a, T>;

        fn into_iter(self) -> AppendListIterator<'a, T> {
            self.iter()
        }
    }

    impl<'a, T> IntoIterator for &'a mut AppendList<T> {
        type Item = &'a mut T;
        type IntoIter = AppendListMutIterator<'a, T>;

        fn into_iter(self) -> AppendListMutIterator<'a, T> {
            self.iter_mut()
        }
    }

    impl<T> Drop for AppendList<T> {
        fn drop(&mut self) {
            unsafe { Self::node_from_raw(mem::replace(self.0.get_mut(), ptr::null_mut())) };
        }
    }

    #[derive(Debug)]
    pub struct AppendListIterator<'a, T: 'a>(&'a AtomicPtr<Node<T>>);

    impl<'a, T: 'a> Iterator for AppendListIterator<'a, T> {
        type Item = &'a T;

        fn next(&mut self) -> Option<&'a T> {
            let p = self.0.load(Ordering::Acquire);
            if p.is_null() {
                None
            } else {
                unsafe {
                    self.0 = &(*p).next.0;
                    Some(&(*p).value)
                }
            }
        }
    }

    #[derive(Debug)]
    pub struct AppendListMutIterator<'a, T: 'a>(&'a mut AtomicPtr<Node<T>>);

    impl<'a, T: 'a> Iterator for AppendListMutIterator<'a, T> {
        type Item = &'a mut T;

        fn next(&mut self) -> Option<&'a mut T> {
            let p = self.0.load(Ordering::Acquire);
            if p.is_null() {
                None
            } else {
                unsafe {
                    self.0 = &mut (*p).next.0;
                    Some(&mut (*p).value)
                }
            }
        }
    }

}

#[cfg(test)]
mod tests {
    use super::UserDataMap;

    #[test]
    fn insert_twice() {
        let map = UserDataMap::new();

        assert_eq!(map.get::<usize>(), None);
        assert!(map.insert_if_missing(|| 42usize));
        assert!(!map.insert_if_missing(|| 43usize));
        assert_eq!(map.get::<usize>(), Some(&42));
    }
}
