use std::{ffi::CStr, sync::Arc};

use wayland_commons::{server::GlobalHandler, Interface};

use super::{ClientId, GlobalId};

pub struct Registry<B> {
    _blah: std::marker::PhantomData<B>,
}

impl<B> Registry<B> {
    pub(crate) fn check_bind(
        &self,
        client: ClientId,
        name: u32,
        interface_name: &CStr,
        version: u32,
    ) -> Option<(&'static Interface, GlobalId, Arc<dyn GlobalHandler<B>>)> {
        todo!()
    }
}
