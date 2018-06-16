//! Wayland objects map

use {Interface, MessageGroup};

/// Limit separating server-created from client-created objects IDs in the namespace
pub const SERVER_ID_LIMIT: u32 = 0xFF000000;

/// The representation of a protocol object
#[derive(Copy, Clone)]
pub struct Object {
    /// Interface name of this object
    pub interface: &'static str,
    /// Version of this object
    pub version: u32,
    /// Description of the requests of this object
    pub requests: &'static [::wire::MessageDesc],
    /// Description of the events of this object
    pub events: &'static [::wire::MessageDesc],
    childs_from_events: fn(u16, u32) -> Option<Object>,
    childs_from_requests: fn(u16, u32) -> Option<Object>,
}

impl Object {
    /// Create an Object corresponding to given interface and version
    pub fn from_interface<I: Interface>(version: u32) -> Object {
        Object {
            interface: I::NAME,
            version: version,
            requests: I::Request::MESSAGES,
            events: I::Event::MESSAGES,
            childs_from_events: childs_from::<I::Event>,
            childs_from_requests: childs_from::<I::Request>,
        }
    }

    /// Create an optional `Object` corresponding to the possible `new_id` associated
    /// with given event opcode
    pub fn event_child(&self, opcode: u16, version: u32) -> Option<Object> {
        (self.childs_from_events)(opcode, version)
    }

    /// Create an optional `Object` corresponding to the possible `new_id` associated
    /// with given request opcode
    pub fn request_child(&self, opcode: u16, version: u32) -> Option<Object> {
        (self.childs_from_requests)(opcode, version)
    }
}

fn childs_from<M: MessageGroup>(opcode: u16, version: u32) -> Option<Object> {
    M::child(opcode, version)
}

/// A holder for the object store of a connection
///
/// Keeps track of which object id is associated to which
/// interface object, and which is currently unused.
pub struct ObjectMap {
    client_objects: Vec<Option<Object>>,
    server_objects: Vec<Option<Object>>,
}

impl ObjectMap {
    /// Create a new empty object map
    pub fn new() -> ObjectMap {
        ObjectMap {
            client_objects: Vec::new(),
            server_objects: Vec::new(),
        }
    }

    /// Find an object in the store
    pub fn find(&self, id: u32) -> Option<Object> {
        if id >= SERVER_ID_LIMIT {
            self.server_objects
                .get((id - SERVER_ID_LIMIT - 1) as usize)
                .and_then(|x| x.clone())
        } else {
            self.client_objects.get((id - 1) as usize).and_then(|x| x.clone())
        }
    }

    /// Delete an object from the store
    ///
    /// Does nothing if the object didn't previously exists
    pub fn delete(&mut self, id: u32) {
        if id >= SERVER_ID_LIMIT {
            if let Some(place) = self.server_objects.get_mut((id - SERVER_ID_LIMIT - 1) as usize) {
                *place = None;
            }
        } else {
            if let Some(place) = self.client_objects.get_mut((id - 1) as usize) {
                *place = None;
            }
        }
    }

    /// Insert given object for given id
    ///
    /// Can fail if the requested id is not the next free id of this store.
    /// (In which case this is a protocol error)
    pub fn insert_at(&mut self, id: u32, object: Object) -> Result<(), ()> {
        if id >= SERVER_ID_LIMIT {
            insert_in_at(&mut self.server_objects, (id - SERVER_ID_LIMIT) as usize, object)
        } else {
            insert_in_at(&mut self.client_objects, id as usize, object)
        }
    }

    /// Allocate a new id for an object in the client namespace
    pub fn client_insert_new(&mut self, object: Object) -> u32 {
        insert_in(&mut self.client_objects, object)
    }

    /// Allocate a new id for an object in the server namespace
    pub fn server_insert_new(&mut self, object: Object) -> u32 {
        insert_in(&mut self.server_objects, object) + SERVER_ID_LIMIT
    }
}

// insert a new object in a store at the first free place
fn insert_in(store: &mut Vec<Option<Object>>, object: Object) -> u32 {
    match store.iter().position(|p| p.is_none()) {
        Some(id) => {
            store[id] = Some(object);
            id as u32 + 1
        }
        None => {
            store.push(Some(object));
            store.len() as u32
        }
    }
}

// insert an object at a given place in a store
fn insert_in_at(store: &mut Vec<Option<Object>>, id: usize, object: Object) -> Result<(), ()> {
    let id = id - 1;
    if id > store.len() {
        Err(())
    } else if id == store.len() {
        store.push(Some(object));
        Ok(())
    } else {
        let place = &mut store[id];
        if place.is_some() {
            return Err(());
        }
        *place = Some(object);
        Ok(())
    }
}
