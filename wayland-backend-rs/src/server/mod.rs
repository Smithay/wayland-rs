use std::{fmt, sync::Arc};

use wayland_commons::{server::ObjectData, Interface};

use crate::same_interface;

mod client;
mod common_poll;
mod handle;
mod independent;
mod registry;

pub use common_poll::CommonPollServerBackend;
pub use handle::Handle;
pub use independent::IndependentServerBackend;

#[derive(Copy, Clone, Debug)]
pub struct ObjectId {
    id: u32,
    serial: u32,
    client_id: ClientId,
    interface: &'static Interface,
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

pub(crate) struct Data<B> {
    user_data: Arc<dyn ObjectData<B>>,
    serial: u32,
}

impl<B> Clone for Data<B> {
    fn clone(&self) -> Data<B> {
        Data { user_data: self.user_data.clone(), serial: self.serial }
    }
}
