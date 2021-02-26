//! Wayland objects map

use wayland_commons::Interface;

use std::cmp::Ordering;

/// Limit separating server-created from client-created objects IDs in the namespace
pub const SERVER_ID_LIMIT: u32 = 0xFF00_0000;

/// The representation of a protocol object
#[derive(Clone)]
pub struct Object<Data> {
    /// Interface name of this object
    pub interface: &'static Interface,
    /// Version of this object
    pub version: u32,
    /// ObjectData associated to this object (ex: its event queue client side)
    pub data: Data,
}

impl<Data> Object<Data> {
    /// Create an Object corresponding to given interface and version
    pub fn from_interface(interface: &'static Interface, version: u32, data: Data) -> Object<Data> {
        Object {
            interface,
            version,
            data,
        }
    }
}

/// A holder for the object store of a connection
///
/// Keeps track of which object id is associated to which
/// interface object, and which is currently unused.
#[derive(Default)]
pub struct ObjectMap<Data> {
    client_objects: Vec<Option<Object<Data>>>,
    server_objects: Vec<Option<Object<Data>>>,
}

impl<Data: Clone> ObjectMap<Data> {
    /// Create a new empty object map
    pub fn new() -> ObjectMap<Data> {
        ObjectMap { client_objects: Vec::new(), server_objects: Vec::new() }
    }

    /// Find an object in the store
    pub fn find(&self, id: u32) -> Option<Object<Data>> {
        if id == 0 {
            None
        } else if id >= SERVER_ID_LIMIT {
            self.server_objects.get((id - SERVER_ID_LIMIT) as usize).and_then(Clone::clone)
        } else {
            self.client_objects.get((id - 1) as usize).and_then(Clone::clone)
        }
    }

    /// Remove an object from the store
    ///
    /// Does nothing if the object didn't previously exists
    pub fn remove(&mut self, id: u32) {
        if id == 0 {
            // nothing
        } else if id >= SERVER_ID_LIMIT {
            if let Some(place) = self.server_objects.get_mut((id - SERVER_ID_LIMIT) as usize) {
                *place = None;
            }
        } else if let Some(place) = self.client_objects.get_mut((id - 1) as usize) {
            *place = None;
        }
    }

    /// Insert given object for given id
    ///
    /// Can fail if the requested id is not the next free id of this store.
    /// (In which case this is a protocol error)
    // -- The lint is allowed because fixing it would be a breaking change --
    #[allow(clippy::result_unit_err)]
    pub fn insert_at(&mut self, id: u32, object: Object<Data>) -> Result<(), ()> {
        if id == 0 {
            Err(())
        } else if id >= SERVER_ID_LIMIT {
            insert_in_at(&mut self.server_objects, (id - SERVER_ID_LIMIT) as usize, object)
        } else {
            insert_in_at(&mut self.client_objects, (id - 1) as usize, object)
        }
    }

    /// Allocate a new id for an object in the client namespace
    pub fn client_insert_new(&mut self, object: Object<Data>) -> u32 {
        insert_in(&mut self.client_objects, object) + 1
    }

    /// Allocate a new id for an object in the server namespace
    pub fn server_insert_new(&mut self, object: Object<Data>) -> u32 {
        insert_in(&mut self.server_objects, object) + SERVER_ID_LIMIT
    }

    /// Mutably access an object of the map
    // -- The lint is allowed because fixing it would be a breaking change --
    #[allow(clippy::result_unit_err)]
    pub fn with<T, F: FnOnce(&mut Object<Data>) -> T>(&mut self, id: u32, f: F) -> Result<T, ()> {
        if id == 0 {
            Err(())
        } else if id >= SERVER_ID_LIMIT {
            if let Some(&mut Some(ref mut obj)) =
                self.server_objects.get_mut((id - SERVER_ID_LIMIT) as usize)
            {
                Ok(f(obj))
            } else {
                Err(())
            }
        } else if let Some(&mut Some(ref mut obj)) = self.client_objects.get_mut((id - 1) as usize)
        {
            Ok(f(obj))
        } else {
            Err(())
        }
    }

    /// Mutably access all objects of the map in sequence
    pub fn with_all<F: FnMut(u32, &mut Object<Data>)>(&mut self, mut f: F) {
        for (id, place) in self.client_objects.iter_mut().enumerate() {
            if let Some(ref mut obj) = *place {
                f(id as u32 + 1, obj);
            }
        }
        for (id, place) in self.server_objects.iter_mut().enumerate() {
            if let Some(ref mut obj) = *place {
                f(id as u32 + SERVER_ID_LIMIT, obj);
            }
        }
    }
}

// insert a new object in a store at the first free place
fn insert_in<Data>(
    store: &mut Vec<Option<Object<Data>>>,
    object: Object<Data>,
) -> u32 {
    match store.iter().position(Option::is_none) {
        Some(id) => {
            store[id] = Some(object);
            id as u32
        }
        None => {
            store.push(Some(object));
            (store.len() - 1) as u32
        }
    }
}

// insert an object at a given place in a store
fn insert_in_at<Data>(
    store: &mut Vec<Option<Object<Data>>>,
    id: usize,
    object: Object<Data>,
) -> Result<(), ()> {
    match id.cmp(&store.len()) {
        Ordering::Greater => Err(()),
        Ordering::Equal => {
            store.push(Some(object));
            Ok(())
        }
        Ordering::Less => {
            let previous = &mut store[id];
            if !previous.is_none() {
                return Err(());
            }
            *previous = Some(object);
            Ok(())
        }
    }
}
