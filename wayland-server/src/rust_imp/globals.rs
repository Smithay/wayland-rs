use {Implementation, Interface, NewResource};

use super::EventLoopInner;

pub(crate) struct GlobalInner<I: Interface> {
    _i: ::std::marker::PhantomData<*const I>,
}

impl<I: Interface> GlobalInner<I> {
    pub fn destroy(self) {
        unimplemented!()
    }
}

pub(crate) struct GlobalManager {}

impl GlobalManager {
    pub(crate) fn new() -> GlobalManager {
        GlobalManager {}
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
}
