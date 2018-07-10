use {Interface, NewResource, Resource};

mod clients;
mod display;
mod event_loop;
mod globals;
mod resources;

pub(crate) use self::clients::ClientInner;
pub(crate) use self::display::DisplayInner;
pub(crate) use self::event_loop::{EventLoopInner, IdleSourceInner, SourceInner};
pub(crate) use self::globals::GlobalInner;
pub(crate) use self::resources::{NewResourceInner, ResourceInner};

pub struct ResourceMap {}

impl ResourceMap {
    pub fn get<I: Interface>(&mut self, id: u32) -> Option<Resource<I>> {
        unimplemented!()
    }
    pub fn get_new<I: Interface>(&mut self, id: u32) -> Option<NewResource<I>> {
        unimplemented!()
    }
}
