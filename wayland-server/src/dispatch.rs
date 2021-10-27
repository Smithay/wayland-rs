use std::sync::Arc;

use wayland_backend::{protocol::ObjectInfo, server::ObjectData};

use crate::{Client, DisplayHandle, Resource};

pub trait Dispatch<I: Resource>: Sized {
    type UserData: DestructionNotify + Default + Send + Sync + 'static;

    fn request(
        &mut self,
        client: &Client,
        resource: &I,
        request: I::Request,
        data: &Self::UserData,
        dhandle: &mut DisplayHandle<'_, Self>,
    );

    fn child_from_request(_: &ObjectInfo) -> Arc<dyn ObjectData<Self>> {
        panic!(
            "Attempting to create an object in event from uninitialized Dispatch<{}>",
            std::any::type_name::<I>()
        );
    }
}

pub trait DestructionNotify {
    fn object_destroyed(&self) {}
}

impl DestructionNotify for () {}

#[macro_export]
macro_rules! generate_child_from_request {
    ($($child_iface:ty),*) => {
        fn child_from_request(info: &$crate::backend::protocol::ObjectInfo) -> std::sync::Arc<dyn $crate::backend::ObjectData<Self>> {
            match () {
                $(
                    () if $crate::backend::protocol::same_interface(info.interface, <$child_iface as $crate::Resource>::interface()) => {
                        std::sync::Arc::new($crate::ResourceData::<$child_iface, <Self as $crate::Dispatch<$child_iface>>::UserData>::default())
                    },
                )*
                _ => panic!("Attempting to create an unexpected object {:?} in event from Dispatch<{}>", info, std::any::type_name::<Self>()),
            }
        }
    }
}

pub struct ResourceData<I, U> {
    marker: std::marker::PhantomData<fn(I)>,
    pub udata: U,
}

impl<I, U: Default> Default for ResourceData<I, U> {
    fn default() -> Self {
        ResourceData { marker: std::marker::PhantomData, udata: Default::default() }
    }
}

impl<
        I: Resource + 'static,
        U: DestructionNotify + Send + Sync + 'static,
        D: Dispatch<I, UserData = U>,
    > ObjectData<D> for ResourceData<I, U>
{
    fn make_child(self: Arc<Self>, _: &mut D, child_info: &ObjectInfo) -> Arc<dyn ObjectData<D>> {
        <D as Dispatch<I>>::child_from_request(child_info)
    }

    fn request(
        &self,
        handle: &mut wayland_backend::server::Handle<D>,
        data: &mut D,
        client_id: wayland_backend::server::ClientId,
        msg: wayland_backend::protocol::Message<wayland_backend::server::ObjectId>,
    ) {
        let mut dhandle = DisplayHandle::from_handle(handle);
        let client = match Client::from_id(&mut dhandle, client_id) {
            Ok(v) => v,
            Err(_) => {
                log::error!("Receiving a request from a dead client ?!");
                return;
            }
        };

        let (resource, request) = match I::parse_request(&mut dhandle, msg) {
            Ok(v) => v,
            Err(e) => {
                log::warn!("Dispatching error encountered: {:?}, killing client.", e);
                // TODO: Kill client
                return;
            }
        };
        let udata = resource.data::<U>().expect("Wrong user_data value for object");

        data.request(&client, &resource, request, udata, &mut &mut dhandle);
    }

    fn destroyed(
        &self,
        _: wayland_backend::server::ClientId,
        _: wayland_backend::server::ObjectId,
    ) {
        self.udata.object_destroyed()
    }
}
