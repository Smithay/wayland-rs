use std::os::unix::io::OwnedFd;
use std::sync::Arc;

use wayland_backend::server::{
    ClientData, ClientId, GlobalHandler, GlobalId, Handle, ObjectData, ObjectId,
};

use crate::{Client, DataInit, DisplayHandle, New, Resource};

pub(crate) struct GlobalData<I, U, D> {
    pub(crate) data: U,
    pub(crate) _types: std::marker::PhantomData<(I, D)>,
}

unsafe impl<I, D, U: Send + Sync> Send for GlobalData<I, U, D> {}
unsafe impl<I, D, U: Send + Sync> Sync for GlobalData<I, U, D> {}

impl<I: Resource + 'static, U: Send + Sync + 'static, D: GlobalDispatch<I, U> + 'static>
    GlobalHandler<D> for GlobalData<I, U, D>
{
    fn can_view(&self, id: ClientId, data: &Arc<dyn ClientData>, _: GlobalId) -> bool {
        let client = Client { id, data: data.clone() };
        <D as GlobalDispatch<I, U>>::can_view(client, &self.data)
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
        let mut protocol_error = None;

        <D as GlobalDispatch<I, U>>::bind(
            data,
            &handle,
            &client,
            New::wrap(resource.clone()),
            &self.data,
            &mut DataInit { store: &mut new_data, error: &mut protocol_error },
        );

        match new_data {
            Some(data) => data,
            None => match protocol_error {
                Some((code, msg)) => {
                    resource.post_error(code, msg);
                    Arc::new(ProtocolErrorData)
                }

                None => panic!(
                    "Bind callback for interface {} did not init new instance.",
                    I::interface().name
                ),
            },
        }
    }
}

struct ProtocolErrorData;

impl<D> ObjectData<D> for ProtocolErrorData {
    fn request(
        self: Arc<Self>,
        _handle: &Handle,
        _data: &mut D,
        _client_id: ClientId,
        _msg: wayland_backend::protocol::Message<ObjectId, OwnedFd>,
    ) -> Option<Arc<dyn ObjectData<D>>> {
        None
    }

    fn destroyed(
        self: Arc<Self>,
        _handle: &Handle,
        _data: &mut D,
        _client_id: ClientId,
        _object_id: ObjectId,
    ) {
    }
}

/// A trait which provides an implementation for handling advertisement of a global to clients with some type
/// of associated user data.
///
/// Its behavior is similar to the [`Dispatch`][crate::Dispatch] trait.
pub trait GlobalDispatch<I: Resource, GlobalData, State = Self>: Sized {
    /// Called when a client has bound this global.
    ///
    /// The return value of this function should contain user data to associate the object created by the
    /// client.
    fn bind(
        state: &mut State,
        handle: &DisplayHandle,
        client: &Client,
        resource: New<I>,
        global_data: &GlobalData,
        data_init: &mut DataInit<'_, State>,
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
    fn can_view(_client: Client, _global_data: &GlobalData) -> bool {
        true
    }
}

/*
 * Dispatch delegation helpers
 */

/// A helper macro which delegates a set of [`GlobalDispatch`] implementations for a resource to some other type which
/// provdes a generic [`GlobalDispatch`] implementation.
///
/// Its usage is similar to the [`delegate_dispatch!()`] macro.
///
/// [`delegate_dispatch!()`]: crate::delegate_dispatch!()
#[macro_export]
macro_rules! delegate_global_dispatch {
    ($(@< $( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+ >)? $dispatch_from:ty : [$interface: ty: $udata: ty] => $dispatch_to: ty) => {
        impl$(< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $crate::GlobalDispatch<$interface, $udata> for $dispatch_from {
            fn bind(
                state: &mut Self,
                dhandle: &$crate::DisplayHandle,
                client: &$crate::Client,
                resource: $crate::New<$interface>,
                global_data: &$udata,
                data_init: &mut $crate::DataInit<'_, Self>,
            ) {
                <$dispatch_to as $crate::GlobalDispatch<$interface, $udata, Self>>::bind(state, dhandle, client, resource, global_data, data_init)
            }

            fn can_view(client: $crate::Client, global_data: &$udata) -> bool {
                <$dispatch_to as $crate::GlobalDispatch<$interface, $udata, Self>>::can_view(client, global_data)
            }
        }
    };
}

#[cfg(test)]
mod tests {
    #[test]
    fn smoke_test_dispatch_global_dispatch() {
        use crate::{
            delegate_dispatch, protocol::wl_output, Client, DataInit, Dispatch, DisplayHandle,
            GlobalDispatch, New,
        };

        struct DelegateToMe;

        impl<D> Dispatch<wl_output::WlOutput, (), D> for DelegateToMe
        where
            D: Dispatch<wl_output::WlOutput, ()> + AsMut<DelegateToMe>,
        {
            fn request(
                _state: &mut D,
                _client: &Client,
                _resource: &wl_output::WlOutput,
                _request: wl_output::Request,
                _data: &(),
                _dhandle: &DisplayHandle,
                _data_init: &mut DataInit<'_, D>,
            ) {
            }
        }
        impl<D> GlobalDispatch<wl_output::WlOutput, (), D> for DelegateToMe
        where
            D: GlobalDispatch<wl_output::WlOutput, ()>,
            D: Dispatch<wl_output::WlOutput, ()>,
            D: AsMut<DelegateToMe>,
        {
            fn bind(
                _state: &mut D,
                _handle: &DisplayHandle,
                _client: &Client,
                _resource: New<wl_output::WlOutput>,
                _global_data: &(),
                _data_init: &mut DataInit<'_, D>,
            ) {
            }
        }

        struct ExampleApp {
            delegate: DelegateToMe,
        }

        delegate_dispatch!(ExampleApp: [wl_output::WlOutput: ()] => DelegateToMe);
        delegate_global_dispatch!(ExampleApp: [wl_output::WlOutput: ()] => DelegateToMe);

        impl AsMut<DelegateToMe> for ExampleApp {
            fn as_mut(&mut self) -> &mut DelegateToMe {
                &mut self.delegate
            }
        }
    }

    #[test]
    fn smoke_test_dispatch_global_dispatch_generics() {
        use crate::{
            delegate_dispatch, protocol::wl_output, Client, DataInit, Dispatch, DisplayHandle,
            GlobalDispatch, New,
        };

        struct DelegateToMe<A>(A);

        impl<A, D> Dispatch<wl_output::WlOutput, (), D> for DelegateToMe<A>
        where
            A: 'static,
            D: Dispatch<wl_output::WlOutput, ()> + AsMut<DelegateToMe<A>>,
        {
            fn request(
                _state: &mut D,
                _client: &Client,
                _resource: &wl_output::WlOutput,
                _request: wl_output::Request,
                _data: &(),
                _dhandle: &DisplayHandle,
                _data_init: &mut DataInit<'_, D>,
            ) {
            }
        }
        impl<A, D> GlobalDispatch<wl_output::WlOutput, (), D> for DelegateToMe<A>
        where
            A: 'static,
            D: GlobalDispatch<wl_output::WlOutput, ()>,
            D: Dispatch<wl_output::WlOutput, ()>,
            D: AsMut<DelegateToMe<A>>,
        {
            fn bind(
                _state: &mut D,
                _handle: &DisplayHandle,
                _client: &Client,
                _resource: New<wl_output::WlOutput>,
                _global_data: &(),
                _data_init: &mut DataInit<'_, D>,
            ) {
            }
        }

        struct ExampleApp<A> {
            delegate: DelegateToMe<A>,
        }

        delegate_dispatch!(@<A: 'static> ExampleApp<A>: [wl_output::WlOutput: ()] => DelegateToMe<A>);
        delegate_global_dispatch!(@<A: 'static> ExampleApp<A>: [wl_output::WlOutput: ()] => DelegateToMe<A>);

        impl<A> AsMut<DelegateToMe<A>> for ExampleApp<A> {
            fn as_mut(&mut self) -> &mut DelegateToMe<A> {
                &mut self.delegate
            }
        }
    }
}
