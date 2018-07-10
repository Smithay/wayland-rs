use {Interface, NewResource, Resource};

mod client;
mod display;
mod event_loop;
mod globals;
mod resource;
mod source;

pub(crate) use self::client::ClientInner;
pub(crate) use self::display::DisplayInner;
pub(crate) use self::event_loop::EventLoopInner;
pub(crate) use self::globals::GlobalInner;
pub(crate) use self::resource::{NewResourceInner, ResourceInner};
pub(crate) use self::source::{IdleSourceInner, SourceInner};

pub enum ResourceMap {}

impl ResourceMap {
    pub fn get<I: Interface>(&mut self, id: u32) -> Option<Resource<I>> {
        match *self {}
    }
    pub fn get_new<I: Interface>(&mut self, id: u32) -> Option<NewResource<I>> {
        match *self {}
    }
}
