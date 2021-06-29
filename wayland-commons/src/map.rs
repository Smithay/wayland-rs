//! Wayland objects map
use crate::{Interface, MessageGroup, NoMessage};

use std::cmp::Ordering;

/// Limit separating server-created from client-created objects IDs in the namespace
pub const SERVER_ID_LIMIT: u32 = 0xFF00_0000;

/// A trait representing the metadata a wayland implementation
/// may attach to an object.
pub trait ObjectMetadata: Clone {
    /// Create the metadata for a child object
    ///
    /// Mostly needed for client side, to propagate the event queues
    fn child(&self) -> Self;
}

impl ObjectMetadata for () {
    fn child(&self) {}
}

/// The representation of a protocol object
#[derive(Clone)]
pub struct Object<Meta: ObjectMetadata> {
    /// Interface name of this object
    pub interface: &'static str,
    /// Version of this object
    pub version: u32,
    /// Description of the requests of this object
    pub requests: &'static [crate::wire::MessageDesc],
    /// Description of the events of this object
    pub events: &'static [crate::wire::MessageDesc],
    /// Metadata associated to this object (ex: its event queue client side)
    pub meta: Meta,
    /// A function which, from an opcode, a version, and the Meta, creates a child
    /// object associated with this event if any
    pub childs_from_events: fn(u16, u32, &Meta) -> Option<Object<Meta>>,
    /// A function which, from an opcode, a version, and the Meta, creates a child
    /// object associated with this request if any
    pub childs_from_requests: fn(u16, u32, &Meta) -> Option<Object<Meta>>,
}

impl<Meta: ObjectMetadata + std::fmt::Debug> std::fmt::Debug for Object<Meta> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Object")
            .field("interface", &self.interface)
            .field("version", &self.version)
            .field("requests", &self.requests)
            .field("events", &self.events)
            .field("meta", &self.meta)
            .finish()
    }
}

impl<Meta: ObjectMetadata> Object<Meta> {
    /// Create an Object corresponding to given interface and version
    pub fn from_interface<I: Interface>(version: u32, meta: Meta) -> Object<Meta> {
        Object {
            interface: I::NAME,
            version,
            requests: I::Request::MESSAGES,
            events: I::Event::MESSAGES,
            meta,
            childs_from_events: childs_from::<I::Event, Meta>,
            childs_from_requests: childs_from::<I::Request, Meta>,
        }
    }

    /// Create an optional `Object` corresponding to the possible `new_id` associated
    /// with given event opcode
    pub fn event_child(&self, opcode: u16) -> Option<Object<Meta>> {
        (self.childs_from_events)(opcode, self.version, &self.meta)
    }

    /// Create an optional `Object` corresponding to the possible `new_id` associated
    /// with given request opcode
    pub fn request_child(&self, opcode: u16) -> Option<Object<Meta>> {
        (self.childs_from_requests)(opcode, self.version, &self.meta)
    }

    /// Check whether this object is of given interface
    pub fn is_interface<I: Interface>(&self) -> bool {
        // TODO: we might want to be more robust than that
        self.interface == I::NAME
    }

    /// Create a placeholder object that will be filled-in by the message logic
    pub fn placeholder(meta: Meta) -> Object<Meta> {
        Object {
            interface: "",
            version: 0,
            requests: &[],
            events: &[],
            meta,
            childs_from_events: childs_from::<NoMessage, Meta>,
            childs_from_requests: childs_from::<NoMessage, Meta>,
        }
    }
}

fn childs_from<M: MessageGroup, Meta: ObjectMetadata>(
    opcode: u16,
    version: u32,
    meta: &Meta,
) -> Option<Object<Meta>> {
    M::child(opcode, version, meta)
}

/// A holder for the object store of a connection
///
/// Keeps track of which object id is associated to which
/// interface object, and which is currently unused.
#[derive(Default, Debug)]
pub struct ObjectMap<Meta: ObjectMetadata> {
    client_objects: Vec<Option<Object<Meta>>>,
    server_objects: Vec<Option<Object<Meta>>>,
}

impl<Meta: ObjectMetadata> ObjectMap<Meta> {
    /// Create a new empty object map
    pub fn new() -> ObjectMap<Meta> {
        ObjectMap { client_objects: Vec::new(), server_objects: Vec::new() }
    }

    /// Find an object in the store
    pub fn find(&self, id: u32) -> Option<Object<Meta>> {
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
    pub fn insert_at(&mut self, id: u32, object: Object<Meta>) -> Result<(), ()> {
        if id == 0 {
            Err(())
        } else if id >= SERVER_ID_LIMIT {
            insert_in_at(&mut self.server_objects, (id - SERVER_ID_LIMIT) as usize, object)
        } else {
            insert_in_at(&mut self.client_objects, (id - 1) as usize, object)
        }
    }

    /// Allocate a new id for an object in the client namespace
    pub fn client_insert_new(&mut self, object: Object<Meta>) -> u32 {
        insert_in(&mut self.client_objects, object) + 1
    }

    /// Allocate a new id for an object in the server namespace
    pub fn server_insert_new(&mut self, object: Object<Meta>) -> u32 {
        insert_in(&mut self.server_objects, object) + SERVER_ID_LIMIT
    }

    /// Mutably access an object of the map
    // -- The lint is allowed because fixing it would be a breaking change --
    #[allow(clippy::result_unit_err)]
    pub fn with<T, F: FnOnce(&mut Object<Meta>) -> T>(&mut self, id: u32, f: F) -> Result<T, ()> {
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
    pub fn with_all<F: FnMut(u32, &mut Object<Meta>)>(&mut self, mut f: F) {
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
fn insert_in<Meta: ObjectMetadata>(
    store: &mut Vec<Option<Object<Meta>>>,
    object: Object<Meta>,
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
fn insert_in_at<Meta: ObjectMetadata>(
    store: &mut Vec<Option<Object<Meta>>>,
    id: usize,
    object: Object<Meta>,
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
