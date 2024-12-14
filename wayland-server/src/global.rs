use std::os::unix::io::OwnedFd;
use std::sync::Arc;

use wayland_backend::server::{
    ClientData, ClientId, GlobalHandler, GlobalId, Handle, ObjectData, ObjectId,
};

use crate::{Client, DataInit, DisplayHandle, New, Resource};

pub(crate) struct GlobalData<I, U, D, DelegatedTo = D> {
    pub(crate) data: U,
    pub(crate) _types: std::marker::PhantomData<(I, D, DelegatedTo)>,
}

unsafe impl<I, D, U: Send + Sync, M> Send for GlobalData<I, U, D, M> {}
unsafe impl<I, D, U: Send + Sync, M> Sync for GlobalData<I, U, D, M> {}

impl<I, U, D, DelegatedTo> GlobalHandler<D> for GlobalData<I, U, D, DelegatedTo>
where
    I: Resource + 'static,
    U: Send + Sync + 'static,
    D: 'static,
    DelegatedTo: GlobalDispatch<I, U, D> + 'static,
{
    fn can_view(&self, id: ClientId, data: &Arc<dyn ClientData>, _: GlobalId) -> bool {
        let client = Client { id, data: data.clone() };
        <DelegatedTo as GlobalDispatch<I, U, D>>::can_view(client, &self.data)
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

        <DelegatedTo as GlobalDispatch<I, U, D>>::bind(
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
