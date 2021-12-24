use std::sync::Arc;

use wayland_backend::server::ObjectData;

use crate::{Client, DisplayHandle, Resource};

/// A trait which provides an implementation for handling a client's requests from a resource with some type
/// of associated user data.
pub trait Dispatch<I: Resource>: Sized {
    /// The user data associated with the type of resource.
    type UserData: DestructionNotify + Send + Sync + 'static;

    /// Called when a request from a client is processed.
    ///
    /// The implementation of this function will vary depending on what protocol is being implemented. Typically
    /// the server may respond to clients by sending events to the resource, or some other resource stored in
    /// the user data.
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

/// A trait to be implemented on user data, which indicates when an object has been destroyed by a client.
pub trait DestructionNotify {
    /// Called when the object this user data is associated with has been destroyed.
    ///
    /// Note this type only provides an immutable reference, you will need to use interior mutability to change
    /// the inside of the object.
    ///
    /// Typically a [`Mutex`](std::sync::Mutex) would be used to have interior mutability.
    #[cfg(not(tarpaulin_include))]
    fn object_destroyed(&self) {}
}

impl DestructionNotify for () {}

#[derive(Debug)]
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

#[derive(Debug)]
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

/*
 * Dispatch delegation helpers.
 */

/// The base trait used to define a delegate type to hand some type of resource.
pub trait DelegateDispatchBase<I: Resource> {
    /// The type of user data the delegate holds.
    type UserData: DestructionNotify + Send + Sync + 'static;
}

/// A trait which defines a delegate to handle some type of resource.
///
/// This trait is useful for building modular handlers of resources.
pub trait DelegateDispatch<
    I: Resource,
    D: Dispatch<I, UserData = <Self as DelegateDispatchBase<I>>::UserData>,
>: Sized + DelegateDispatchBase<I>
{
    /// Called when a request from a client is processed.
    ///
    /// The implementation of this function will vary depending on what protocol is being implemented. Typically
    /// the server may respond to clients by sending events to the resource, or some other resource stored in
    /// the user data.
    fn request(
        &mut self,
        client: &Client,
        resource: &I,
        request: I::Request,
        data: &Self::UserData,
        dhandle: &mut DisplayHandle<D>,
        data_init: &mut DataInit<'_, D>,
    );
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
