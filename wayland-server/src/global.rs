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
        handle: &Handle,
        data: &mut D,
        client_id: ClientId,
        _: GlobalId,
        object_id: ObjectId,
    ) -> Arc<dyn ObjectData<D>> {
        let handle = DisplayHandle::from(handle.clone());
        let client = Client::from_id(&handle, client_id).expect("Dead client in bind ?!");
        let resource = <I as Resource>::from_id(&handle, object_id)
            .expect("Wrong object_id in GlobalHandler ?!");

        let mut new_data = None;

        data.bind(
            &handle,
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
        handle: &DisplayHandle,
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
        handle: &DisplayHandle,
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
    (@impl $dispatch_from:ident $(< $( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+ >)? : $interface: ty => $dispatch_to: ty) => {
        impl$(< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $crate::GlobalDispatch<$interface> for $dispatch_from$(< $( $lt ),+ >)? {
            type GlobalData = <$dispatch_to as $crate::DelegateGlobalDispatchBase<$interface>>::GlobalData;

            fn bind(
                &mut self,
                dhandle: &$crate::DisplayHandle,
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
    };
    ($impl:tt : [$($interface: ty),*] => $dispatch_to: ty) => {
        $(
            $crate::delegate_global_dispatch!(@impl $impl : $interface => $dispatch_to);
        )*
    };
}

#[cfg(test)]
mod tests {
    #[test]
    fn smoke_test_dispatch_global_dispatch() {
        use crate::{
            delegate_dispatch, protocol::wl_output, Client, DataInit, DelegateDispatch,
            DelegateDispatchBase, DelegateGlobalDispatch, DelegateGlobalDispatchBase, Dispatch,
            DisplayHandle, GlobalDispatch, New,
        };

        struct DelegateToMe;

        impl DelegateDispatchBase<wl_output::WlOutput> for DelegateToMe {
            type UserData = ();
        }
        impl<D> DelegateDispatch<wl_output::WlOutput, D> for DelegateToMe
        where
            D: Dispatch<wl_output::WlOutput, UserData = Self::UserData> + AsMut<DelegateToMe>,
        {
            fn request(
                _state: &mut D,
                _client: &Client,
                _resource: &wl_output::WlOutput,
                _request: wl_output::Request,
                _data: &Self::UserData,
                _dhandle: &DisplayHandle,
                _data_init: &mut DataInit<'_, D>,
            ) {
            }
        }
        impl DelegateGlobalDispatchBase<wl_output::WlOutput> for DelegateToMe {
            type GlobalData = ();
        }
        impl<D> DelegateGlobalDispatch<wl_output::WlOutput, D> for DelegateToMe
        where
            D: GlobalDispatch<wl_output::WlOutput, GlobalData = Self::GlobalData>,
            D: Dispatch<wl_output::WlOutput, UserData = ()>,
            D: AsMut<DelegateToMe>,
        {
            fn bind(
                _state: &mut D,
                _handle: &DisplayHandle,
                _client: &Client,
                _resource: New<wl_output::WlOutput>,
                _global_data: &Self::GlobalData,
                _data_init: &mut DataInit<'_, D>,
            ) {
            }
        }

        struct ExampleApp {
            delegate: DelegateToMe,
        }

        delegate_dispatch!(ExampleApp: [wl_output::WlOutput] => DelegateToMe);
        delegate_global_dispatch!(ExampleApp: [wl_output::WlOutput] => DelegateToMe);

        impl AsMut<DelegateToMe> for ExampleApp {
            fn as_mut(&mut self) -> &mut DelegateToMe {
                &mut self.delegate
            }
        }
    }
}
