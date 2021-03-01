use std::{fmt, sync::Arc};

use wayland_commons::{server::ObjectData, Interface};

mod client;
mod handle;
mod registry;

#[derive(Copy, Clone, Debug)]
pub struct ObjectId {
    id: u32,
    client_id: u32,
    serial: u32,
    interface: &'static Interface,
}

impl fmt::Display for ObjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}@{}[{}]", self.interface.name, self.id, self.client_id)
    }
}

#[derive(Copy, Clone, Debug)]
pub struct ClientId {
    id: u32,
    serial: u32,
}

impl fmt::Display for ClientId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}]", self.id)
    }
}
#[derive(Copy, Clone, Debug)]
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
