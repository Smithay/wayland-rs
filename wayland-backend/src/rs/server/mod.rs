//! Server-side rust implementation of a Wayland protocol backend

use std::{fmt, sync::Arc};

use crate::protocol::{same_interface, Interface, Message};

mod client;
mod common_poll;
mod handle;
mod registry;

pub use crate::types::server::{Credentials, DisconnectReason, GlobalInfo, InitError, InvalidId};
pub use common_poll::Backend;
pub use handle::Handle;

/// A trait representing your data associated to an object
///
/// You will only be given access to it as a `&` reference, so you
/// need to handle interior mutability by yourself.
///
/// The methods of this trait will be invoked internally every time a
/// new object is created to initialize its data.
pub trait ObjectData<D>: downcast_rs::DowncastSync {
    /// Dispatch a request for the associated object
    ///
    /// If the request has a NewId argument, the callback must return the object data
    /// for the newly created object
    fn request(
        self: Arc<Self>,
        handle: &mut Handle<D>,
        data: &mut D,
        client_id: ClientId,
        msg: Message<ObjectId>,
    ) -> Option<Arc<dyn ObjectData<D>>>;
    /// Notification that the object has been destroyed and is no longer active
    fn destroyed(&self, client_id: ClientId, object_id: ObjectId);
    /// Helper for forwarding a Debug implementation of your `ObjectData` type
    ///
    /// By default will just print `ObjectData { ... }`
    fn debug(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ObjectData").finish_non_exhaustive()
    }
}

downcast_rs::impl_downcast!(sync ObjectData<D>);

#[cfg(not(tarpaulin_include))]
impl<D> std::fmt::Debug for dyn ObjectData<D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.debug(f)
    }
}

/// A trait representing the handling of new bound globals
pub trait GlobalHandler<D>: downcast_rs::DowncastSync {
    /// Check if given client is allowed to interact with given global
    ///
    /// If this function returns false, the client will not be notified of the existence
    /// of this global, and any attempt to bind it will result in a protocol error as if
    /// the global did not exist.
    ///
    /// Default implementation always return true.
    fn can_view(
        &self,
        _client_id: ClientId,
        _client_data: &Arc<dyn ClientData<D>>,
        _global_id: GlobalId,
    ) -> bool {
        true
    }
    /// A global has been bound
    ///
    /// Given client bound given global, creating given object.
    ///
    /// The method must return the object data for the newly created object.
    fn bind(
        self: Arc<Self>,
        handle: &mut Handle<D>,
        data: &mut D,
        client_id: ClientId,
        global_id: GlobalId,
        object_id: ObjectId,
    ) -> Arc<dyn ObjectData<D>>;
    /// Helper for forwarding a Debug implementation of your `GlobalHandler` type
    ///
    /// By default will just print `GlobalHandler { ... }`
    fn debug(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GlobalHandler").finish_non_exhaustive()
    }
}

#[cfg(not(tarpaulin_include))]
impl<D> std::fmt::Debug for dyn GlobalHandler<D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.debug(f)
    }
}

downcast_rs::impl_downcast!(sync GlobalHandler<D>);

/// A trait representing your data associated to a clientObjectData
pub trait ClientData<D>: downcast_rs::DowncastSync {
    /// Notification that a client was initialized
    fn initialized(&self, client_id: ClientId);

    /// Notification that a client is disconnected
    fn disconnected(&self, client_id: ClientId, reason: DisconnectReason);
    /// Helper for forwarding a Debug implementation of your `ClientData` type
    ///
    /// By default will just print `GlobalHandler { ... }`
    fn debug(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClientData").finish_non_exhaustive()
    }
}

#[cfg(not(tarpaulin_include))]
impl<D> std::fmt::Debug for dyn ClientData<D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.debug(f)
    }
}

downcast_rs::impl_downcast!(sync ClientData<D>);

/// An id of an object on a wayland server.
#[derive(Clone)]
pub struct ObjectId {
    id: u32,
    serial: u32,
    client_id: ClientId,
    interface: &'static Interface,
}

impl ObjectId {
    /// Returns whether this object is a null object.
    pub fn is_null(&self) -> bool {
        self.id == 0
    }

    /// Returns the interface of this object.
    pub fn interface(&self) -> &'static Interface {
        self.interface
    }

    /// Check if two object IDs are associated with the same client
    ///
    /// *Note:* This may spuriously return `false` if one (or both) of the objects to compare
    /// is no longer valid.
    pub fn same_client_as(&self, other: &ObjectId) -> bool {
        self.client_id == other.client_id
    }

    /// Return the protocol-level numerical ID of this object
    ///
    /// Protocol IDs are reused after object destruction, so this should not be used as a
    /// unique identifier,
    pub fn protocol_id(&self) -> u32 {
        self.id
    }
}

#[cfg(not(tarpaulin_include))]
impl fmt::Display for ObjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}@{}[{}]", self.interface.name, self.id, self.client_id)
    }
}

#[cfg(not(tarpaulin_include))]
impl fmt::Debug for ObjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ObjectId({}, {})", self, self.serial)
    }
}

impl PartialEq for ObjectId {
    fn eq(&self, other: &ObjectId) -> bool {
        self.id == other.id
            && self.serial == other.serial
            && self.client_id == other.client_id
            && same_interface(self.interface, other.interface)
    }
}

impl std::cmp::Eq for ObjectId {}

/// An id of a client connected to the server.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClientId {
    id: u32,
    serial: u32,
}

impl ClientId {
    fn as_u64(&self) -> u64 {
        ((self.id as u64) << 32) + self.serial as u64
    }

    fn from_u64(t: u64) -> Self {
        Self { id: (t >> 32) as u32, serial: t as u32 }
    }
}

#[cfg(not(tarpaulin_include))]
impl fmt::Display for ClientId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.id)
    }
}

/// The ID of a global
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GlobalId {
    id: u32,
    serial: u32,
}

#[derive(Debug)]
pub(crate) struct Data<D> {
    user_data: Arc<dyn ObjectData<D>>,
    serial: u32,
}

#[cfg(not(tarpaulin_include))]
impl<D> Clone for Data<D> {
    fn clone(&self) -> Data<D> {
        Data { user_data: self.user_data.clone(), serial: self.serial }
    }
}

struct UninitObjectData;

impl<D> ObjectData<D> for UninitObjectData {
    fn request(
        self: Arc<Self>,
        _: &mut Handle<D>,
        _: &mut D,
        _: ClientId,
        msg: Message<ObjectId>,
    ) -> Option<Arc<dyn ObjectData<D>>> {
        panic!("Received a message on an uninitialized object: {:?}", msg);
    }

    fn destroyed(&self, _: ClientId, _: ObjectId) {}

    fn debug(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UninitObjectData").finish()
    }
}
