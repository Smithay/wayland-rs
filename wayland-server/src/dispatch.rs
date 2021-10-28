use std::sync::Arc;

use wayland_backend::server::ObjectData;

use crate::{Client, DisplayHandle, Resource};

pub trait Dispatch<I: Resource>: Sized {
    type UserData: DestructionNotify + Send + Sync + 'static;

    fn request(
        &mut self,
        client: &Client,
        resource: &I,
        request: I::Request,
        data: &Self::UserData,
        dhandle: &mut DisplayHandle<'_, Self>,
        data_init: &mut DataInit<'_, Self>,
    );
}

pub trait DestructionNotify {
    fn object_destroyed(&self) {}
}

impl DestructionNotify for () {}

pub struct ResourceData<I, U> {
    marker: std::marker::PhantomData<fn(I)>,
    pub udata: U,
}

#[derive(Debug)]
pub struct New<I> {
    id: I,
}

impl<I> New<I> {
    pub fn wrap(id: I) -> New<I> {
        New { id }
    }
}

pub struct DataInit<'a, D> {
    store: &'a mut Option<Arc<dyn ObjectData<D>>>,
}

impl<'a, D> DataInit<'a, D> {
    pub fn init<I: Resource + 'static>(
        &mut self,
        resource: New<I>,
        data: <D as Dispatch<I>>::UserData,
    ) -> I
    where
        D: Dispatch<I>,
    {
        *self.store = Some(Arc::new(ResourceData::<I, _>::new(data)));
        resource.id
    }
}

impl<I, U> ResourceData<I, U> {
    pub(crate) fn new(udata: U) -> Self {
        ResourceData { marker: std::marker::PhantomData, udata }
    }
}

impl<
        I: Resource + 'static,
        U: DestructionNotify + Send + Sync + 'static,
        D: Dispatch<I, UserData = U>,
    > ObjectData<D> for ResourceData<I, U>
{
    fn request(
        self: Arc<Self>,
        handle: &mut wayland_backend::server::Handle<D>,
        data: &mut D,
        client_id: wayland_backend::server::ClientId,
        msg: wayland_backend::protocol::Message<wayland_backend::server::ObjectId>,
    ) -> Option<Arc<dyn ObjectData<D>>> {
        let mut dhandle = DisplayHandle::from_handle(handle);
        let client = match Client::from_id(&mut dhandle, client_id) {
            Ok(v) => v,
            Err(_) => {
                log::error!("Receiving a request from a dead client ?!");
                return None;
            }
        };

        let (resource, request) = match I::parse_request(&mut dhandle, msg) {
            Ok(v) => v,
            Err(e) => {
                log::warn!("Dispatching error encountered: {:?}, killing client.", e);
                // TODO: Kill client
                return None;
            }
        };
        let udata = resource.data::<U>().expect("Wrong user_data value for object");

        let mut new_data = None;

        data.request(
            &client,
            &resource,
            request,
            udata,
            &mut dhandle,
            &mut DataInit { store: &mut new_data },
        );

        new_data
    }

    fn destroyed(
        &self,
        _: wayland_backend::server::ClientId,
        _: wayland_backend::server::ObjectId,
    ) {
        self.udata.object_destroyed()
    }
}
