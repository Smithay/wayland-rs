//! Wayland objects map

use {Interface, MessageGroup};

/// Limit separating server-created from client-created objects IDs in the namespace
pub const SERVER_ID_LIMIT: u32 = 0xFF000000;

/// The representation of a protocol object
#[derive(Clone)]
pub struct Object<Meta: Clone> {
    /// Interface name of this object
    pub interface: &'static str,
    /// Version of this object
    pub version: u32,
    /// Description of the requests of this object
    pub requests: &'static [::wire::MessageDesc],
    /// Description of the events of this object
    pub events: &'static [::wire::MessageDesc],
    /// True if this is a zombie, a defunct object
    pub zombie: bool,
    /// Metadata associated to this object (ex: its event queue client side)
    pub meta: Meta,
    childs_from_events: fn(u16, u32, &Meta) -> Option<Object<Meta>>,
    childs_from_requests: fn(u16, u32, &Meta) -> Option<Object<Meta>>,
}

impl<Meta: Clone> Object<Meta> {
    /// Create an Object corresponding to given interface and version
    pub fn from_interface<I: Interface>(version: u32, meta: Meta) -> Object<Meta> {
        Object {
            interface: I::NAME,
            version: version,
            requests: I::Request::MESSAGES,
            events: I::Event::MESSAGES,
            zombie: false,
            meta: meta,
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
}

fn childs_from<M: MessageGroup, Meta: Clone>(opcode: u16, version: u32, meta: &Meta) -> Option<Object<Meta>> {
    M::child(opcode, version, meta)
}

/// A holder for the object store of a connection
///
/// Keeps track of which object id is associated to which
/// interface object, and which is currently unused.
pub struct ObjectMap<Meta: Clone> {
    client_objects: Vec<Object<Meta>>,
    server_objects: Vec<Object<Meta>>,
}

impl<Meta: Clone> ObjectMap<Meta> {
    /// Create a new empty object map
    pub fn new() -> ObjectMap<Meta> {
        ObjectMap {
            client_objects: Vec::new(),
            server_objects: Vec::new(),
        }
    }

    /// Find an object in the store
    pub fn find(&self, id: u32) -> Option<Object<Meta>> {
        if id >= SERVER_ID_LIMIT {
            self.server_objects
                .get((id - SERVER_ID_LIMIT - 1) as usize)
                .cloned()
        } else {
            self.client_objects.get((id - 1) as usize).cloned()
        }
    }

    /// Find a non-zombie object in the store
    pub fn find_alive(&self, id: u32) -> Option<Object<Meta>> {
        self.find(id).and_then(|o| if !o.zombie { Some(o) } else { None })
    }

    /// Marks an object from the store as dead
    ///
    /// It marks its field `zombie = true`.
    ///
    /// Does nothing if the object didn't previously exists
    pub fn kill(&mut self, id: u32) {
        if id >= SERVER_ID_LIMIT {
            if let Some(place) = self.server_objects.get_mut((id - SERVER_ID_LIMIT - 1) as usize) {
                place.zombie = true;
            }
        } else {
            if let Some(place) = self.client_objects.get_mut((id - 1) as usize) {
                place.zombie = true;
            }
        }
    }

    /// Insert given object for given id
    ///
    /// Can fail if the requested id is not the next free id of this store.
    /// (In which case this is a protocol error)
    pub fn insert_at(&mut self, id: u32, object: Object<Meta>) -> Result<(), ()> {
        if id >= SERVER_ID_LIMIT {
            insert_in_at(&mut self.server_objects, (id - SERVER_ID_LIMIT) as usize, object)
        } else {
            insert_in_at(&mut self.client_objects, id as usize, object)
        }
    }

    /// Allocate a new id for an object in the client namespace
    pub fn client_insert_new(&mut self, object: Object<Meta>) -> u32 {
        insert_in(&mut self.client_objects, object)
    }

    /// Allocate a new id for an object in the server namespace
    pub fn server_insert_new(&mut self, object: Object<Meta>) -> u32 {
        insert_in(&mut self.server_objects, object) + SERVER_ID_LIMIT
    }
}

// insert a new object in a store at the first free place
fn insert_in<Meta: Clone>(store: &mut Vec<Object<Meta>>, object: Object<Meta>) -> u32 {
    match store.iter().position(|o| o.zombie) {
        Some(id) => {
            store[id] = object;
            id as u32 + 1
        }
        None => {
            store.push(object);
            store.len() as u32
        }
    }
}

// insert an object at a given place in a store
fn insert_in_at<Meta: Clone>(
    store: &mut Vec<Object<Meta>>,
    id: usize,
    object: Object<Meta>,
) -> Result<(), ()> {
    let id = id - 1;
    if id > store.len() {
        Err(())
    } else if id == store.len() {
        store.push(object);
        Ok(())
    } else {
        let previous = &mut store[id];
        if !previous.zombie {
            return Err(());
        }
        *previous = object;
        Ok(())
    }
}
