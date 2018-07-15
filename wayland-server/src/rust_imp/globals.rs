use {Implementation, Interface, NewResource};

use super::{ClientInner, EventLoopInner};

pub(crate) struct GlobalInner<I: Interface> {
    _i: ::std::marker::PhantomData<*const I>,
}

impl<I: Interface> GlobalInner<I> {
    pub fn destroy(self) {
        unimplemented!()
    }
}

pub(crate) struct GlobalManager {
    registries: Vec<(u32, ClientInner)>,
}

impl GlobalManager {
    pub(crate) fn new() -> GlobalManager {
        GlobalManager {
            registries: Vec::new(),
        }
    }

    pub(crate) fn add_global<I: Interface, Impl>(
        &mut self,
        event_loop: &EventLoopInner,
        version: u32,
        implementation: Impl,
    ) -> GlobalInner<I>
    where
        Impl: Implementation<NewResource<I>, u32> + 'static,
    {
        unimplemented!()
    }

    pub(crate) fn new_registry(&mut self, id: u32, client: ClientInner) {
        unimplemented!()
    }

    pub(crate) fn bind(
        &self,
        resource_newid: u32,
        global_id: u32,
        interface: &str,
        version: u32,
        client: ClientInner,
    ) -> Result<(), ()> {
        unimplemented!()
    }
}
