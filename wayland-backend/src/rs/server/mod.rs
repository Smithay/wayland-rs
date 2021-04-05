use std::{fmt, sync::Arc};

use crate::protocol::{Interface, Message, ObjectInfo};
use crate::types::same_interface;

mod client;
mod common_poll;
mod handle;
mod registry;

pub use crate::types::server::{DisconnectReason, GlobalInfo, InitError, InvalidId};
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
    /// Create a new object data from the parent data
    fn make_child(self: Arc<Self>, data: &mut D, child_info: &ObjectInfo)
        -> Arc<dyn ObjectData<D>>;
    /// Dispatch a request for the associated object
    fn request(
        &self,
        handle: &mut Handle<D>,
        data: &mut D,
        client_id: ClientId,
        msg: Message<ObjectId>,
    );
    /// Notification that the object has been destroyed and is no longer active
    fn destroyed(&self, client_id: ClientId, object_id: ObjectId);
}

downcast_rs::impl_downcast!(sync ObjectData<D>);

/// A trait representing the handling of new bound globals
pub trait GlobalHandler<D>: downcast_rs::DowncastSync {
    /// Check if given client is allowed to interact with given global
    ///
    /// If this function returns false, the client will not be notified of the existence
    /// of this global, and any attempt to bind it will result in a protocol error as if
    /// the global did not exist.
    ///
    /// Default implementation always return true.
    fn can_view(&self, _client_id: ClientId, _global_id: GlobalId) -> bool {
        true
    }
    /// Create the ObjectData for a future bound global
    fn make_data(self: Arc<Self>, data: &mut D, info: &ObjectInfo) -> Arc<dyn ObjectData<D>>;
    /// A global has been bound
    ///
    /// Given client bound given global, creating given object.
    fn bind(
        &self,
        handle: &mut Handle<D>,
        data: &mut D,
        client_id: ClientId,
        global_id: GlobalId,
        object_id: ObjectId,
    );
}

downcast_rs::impl_downcast!(sync GlobalHandler<D>);

/// A trait representing your data associated to a clientObjectData
pub trait ClientData<D>: downcast_rs::DowncastSync {
    /// Notification that a client was initialized
    fn initialized(&self, client_id: ClientId);

    /// Notification that a client is disconnected
    fn disconnected(&self, client_id: ClientId, reason: DisconnectReason);
}

downcast_rs::impl_downcast!(sync ClientData<D>);

#[derive(Copy, Clone, Debug)]
pub struct ObjectId {
    id: u32,
    serial: u32,
    client_id: ClientId,
    interface: &'static Interface,
}

impl ObjectId {
    pub fn is_null(&self) -> bool {
        self.id == 0
    }
}

impl fmt::Display for ObjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}@{}[{}]", self.interface.name, self.id, self.client_id)
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

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ClientId {
    id: u32,
    serial: u32,
}

impl ClientId {
    fn as_u64(self) -> u64 {
        ((self.id as u64) << 32) + self.serial as u64
    }

    fn from_u64(t: u64) -> Self {
        Self { id: (t >> 32) as u32, serial: t as u32 }
    }
}

impl fmt::Display for ClientId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}]", self.id)
    }
}
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct GlobalId {
    id: u32,
    serial: u32,
}

pub(crate) struct Data<D> {
    user_data: Arc<dyn ObjectData<D>>,
    serial: u32,
}

impl<D> Clone for Data<D> {
    fn clone(&self) -> Data<D> {
        Data { user_data: self.user_data.clone(), serial: self.serial }
    }
}
