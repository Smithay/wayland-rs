use std::sync::Arc;

use wayland_backend::server::{
    ClientData, ClientId, GlobalHandler, GlobalId, Handle, ObjectData, ObjectId,
};

use crate::{
    dispatch::{DelegateDispatch, DelegateDispatchBase},
    Client, DataInit, Dispatch, DisplayHandle, New, Resource,
};

pub(crate) struct GlobalData<I: Resource, D: GlobalDispatch<I>> {
    pub(crate) data: <D as GlobalDispatch<I>>::GlobalData,
}

impl<I: Resource + 'static, D: GlobalDispatch<I> + 'static> GlobalHandler<D> for GlobalData<I, D> {
    fn can_view(&self, id: ClientId, data: &Arc<dyn ClientData<D>>, _: GlobalId) -> bool {
        let client = Client { id, data: data.clone().into_any_arc() };
        <D as GlobalDispatch<I>>::can_view(client, &self.data)
    }

    fn bind(
        self: Arc<Self>,
        handle: &mut Handle<D>,
        data: &mut D,
        client_id: ClientId,
        _: GlobalId,
        object_id: ObjectId,
    ) -> Arc<dyn ObjectData<D>> {
        let mut handle = DisplayHandle::from_handle(handle);
        let client = Client::from_id(&mut handle, client_id).expect("Dead client in bind ?!");
        let resource = <I as Resource>::from_id(&mut handle, object_id)
            .expect("Wrong object_id in GlobalHandler ?!");

        let mut new_data = None;

        data.bind(
            &mut handle,
            &client,
            New::wrap(resource),
            &self.data,
            &mut DataInit { store: &mut new_data },
        );

        match new_data {
            Some(data) => data,
            None => panic!(
                "Bind callback for interface {} did not init new instance.",
                I::interface().name
            ),
        }
    }
}

/// A trait which provides an implementation for handling advertisement of a global to clients with some type
/// of associated user data.
pub trait GlobalDispatch<I: Resource>: Dispatch<I> {
    /// Data associated with the global.
    type GlobalData: Send + Sync + 'static;

    /// Called when a client has bound this global.
    ///
    /// The return value of this function should contain user data to associate the object created by the
    /// client.
    fn bind(
        &mut self,
        handle: &mut DisplayHandle<'_>,
        client: &Client,
        resource: New<I>,
        global_data: &Self::GlobalData,
        data_init: &mut DataInit<'_, Self>,
    );

    /// Checks if the global should be advertised to some client.
    ///
    /// The implementation of this function determines whether a client may see and bind some global. If this
    /// function returns false, the client will not be told the global exists and attempts to bind the global
    /// will raise a protocol error.
    ///
    /// One use of this function is implementing privileged protocols such as XWayland keyboard grabbing
    /// which must only be used by XWayland.
    ///
    /// The default implementation allows all clients to see the global.
    fn can_view(_client: Client, _global_data: &Self::GlobalData) -> bool {
        true
    }
}

/*
 * Dispatch delegation helpers
 */

/// The base trait used to define a delegate type to handle some global.
pub trait DelegateGlobalDispatchBase<I: Resource>: DelegateDispatchBase<I> {
    /// Data associated with the global.
    type GlobalData: Send + Sync + 'static;
}

/// A trait which defines a delegate type to handle some type of global.
///
/// This trait is useful for building modular handlers for globals.
pub trait DelegateGlobalDispatch<
    I: Resource,
    D: GlobalDispatch<I, GlobalData = <Self as DelegateGlobalDispatchBase<I>>::GlobalData>
        + Dispatch<I, UserData = Self::UserData>,
>: Sized + DelegateGlobalDispatchBase<I> + DelegateDispatch<I, D>
{
    /// Called when a client has bound this global.
    ///
    /// The return value of this function should contain user data to associate the object created by the
    /// client.
    fn bind(
        state: &mut D,
        handle: &mut DisplayHandle<'_>,
        client: &Client,
        resource: New<I>,
        global_data: &Self::GlobalData,
        data_init: &mut DataInit<'_, D>,
    );

    /// Checks if the global should be advertised to some client.
    ///
    /// The implementation of this function determines whether a client may see and bind some global. If this
    /// function returns false, the client will not be told the global exists and attempts to bind the global
    /// will raise a protocol error.
    ///
    /// One use of this function is implementing privileged protocols such as XWayland keyboard grabbing
    /// which must only be used by XWayland.
    ///
    /// The default implementation allows all clients to see the global.
    fn can_view(_client: Client, _global_data: &Self::GlobalData) -> bool {
        true
    }
}

#[macro_export]
macro_rules! delegate_global_dispatch {
    ($dispatch_from: ty: [$($interface: ty),*] => $dispatch_to: ty) => {
        $(
            impl $crate::GlobalDispatch<$interface> for $dispatch_from {
                type GlobalData = <$dispatch_to as $crate::DelegateGlobalDispatchBase<$interface>>::GlobalData;

                fn bind(
                    &mut self,
                    dhandle: &mut $crate::DisplayHandle<'_, Self>,
                    client: &$crate::Client,
                    resource: $crate::New<$interface>,
                    global_data: &Self::GlobalData,
                    data_init: &mut $crate::DataInit<'_, Self>,
                ) {
                    <$dispatch_to as $crate::DelegateGlobalDispatch<$interface, Self>>::bind(self, dhandle, client, resource, global_data, data_init)
                }

                fn can_view(client: $crate::Client, global_data: &Self::GlobalData) -> bool {
                    <$dispatch_to as $crate::DelegateGlobalDispatch<$interface, Self>>::can_view(client, global_data)
                }
            }
        )*
    };
}
