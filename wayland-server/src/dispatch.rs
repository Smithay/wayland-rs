use std::sync::Arc;

use wayland_backend::server::{ClientId, ObjectData, ObjectId};

use crate::{Client, DisplayHandle, Resource};

/// A trait which provides an implementation for handling a client's requests from a resource with some type
/// of associated user data.
///
///  ## General usage
///
/// You need to implement this trait on your `State` for every type of Wayland object that will be processed
/// by the [`Display`](crate::Display) working with your `State`.
///
/// You can have different implementations of the trait for the same interface but different `UserData` type,
/// this way the events for a given object will be processed by the adequate implementation depending on
/// which `UserData` was assigned to it at creation.
///
/// The way this trait works is that the [`Dispatch::request()`] method will be invoked by the
/// [`Display`](crate::Display) for every request received by an object. Your implementation can then match
/// on the associated [`Resource::Request`] enum and do any processing needed with that event.
///
/// ## Modularity
///
/// To provide generic handlers for downstream usage, it is possible to make an implementation of the trait
/// that is generic over the last type argument, as illustrated below. Users will then be able to
/// automatically delegate their implementation to yours using the [`delegate_dispatch!`] macro.
///
/// As a result, when your implementation is instanciated, the last type parameter `State` will be the state
/// struct of the app using your generic implementation. You can put additional trait constraints on it to
/// specify an interface between your module and downstream code, as illustrated in this example:
///
/// ```
/// # // Maintainers: If this example changes, please make sure you also carry those changes over to the delegate_dispatch macro.
/// use wayland_server::{protocol::wl_output, Dispatch};
///
/// /// The type we want to delegate to
/// struct DelegateToMe;
///
/// /// The user data relevant for your implementation.
/// /// When providing delegate implementation, it is recommended to use your own type here, even if it is
/// /// just a unit struct: using () would cause a risk of clashing with an other such implementation.
/// struct MyUserData;
///
/// // Now a generic implementation of Dispatch, we are generic over the last type argument instead of using
/// // the default State=Self.
/// impl<State> Dispatch<wl_output::WlOutput, MyUserData, State> for DelegateToMe
/// where
///     // State is the type which has delegated to this type, so it needs to have an impl of Dispatch itself
///     State: Dispatch<wl_output::WlOutput, MyUserData>,
///     // If your delegate type has some internal state, it'll need to access it, and you can
///     // require it by adding custom trait bounds.
///     // In this example, we just require an AsMut implementation
///     State: AsMut<DelegateToMe>,
/// {
///     fn request(
///         state: &mut State,
///         _client: &wayland_server::Client,
///         _resource: &wl_output::WlOutput,
///         _request: wl_output::Request,
///         _udata: &MyUserData,
///         _dhandle: &wayland_server::DisplayHandle,
///         _data_init: &mut wayland_server::DataInit<'_, State>,
///     ) {
///         // Here the delegate may handle incoming requests as it pleases.
///
///         // For example, it retrives its state and does some processing with it
///         let me: &mut DelegateToMe = state.as_mut();
///         // do something with `me` ...
/// #       std::mem::drop(me) // use `me` to avoid a warning
///     }
/// }
/// ```
///
/// **Note:** Due to limitations in Rust's trait resolution algorithm, a type providing a generic
/// implementation of [`Dispatch`] cannot be used directly as the dispatching state, as rustc
/// currently fails to understand that it also provides `Dispatch<I, U, Self>` (assuming all other
/// trait bounds are respected as well).
pub trait Dispatch<I: Resource, UserData, State = Self>: Sized {
    /// Called when a request from a client is processed.
    ///
    /// The implementation of this function will vary depending on what protocol is being implemented. Typically
    /// the server may respond to clients by sending events to the resource, or some other resource stored in
    /// the user data.
    fn request(
        state: &mut State,
        client: &Client,
        resource: &I,
        request: I::Request,
        data: &UserData,
        dhandle: &DisplayHandle,
        data_init: &mut DataInit<'_, State>,
    );

    /// Called when the object this user data is associated with has been destroyed.
    ///
    /// Note this type only provides an immutable reference to the user data, you will need to use
    /// interior mutability to change it.
    ///
    /// Typically a [`Mutex`](std::sync::Mutex) would be used to have interior mutability.
    ///
    /// You are given the [`ObjectId`] and [`ClientId`] associated with the destroyed object for cleanup
    /// convenience.
    ///
    /// By default this method does nothing.
    fn destroyed(_state: &mut State, _client: ClientId, _resource: ObjectId, _data: &UserData) {}
}

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
    #[doc(hidden)]
    // This is only to be used by code generated by wayland-scanner
    pub fn wrap(id: I) -> New<I> {
        New { id }
    }
}

#[derive(Debug)]
pub struct DataInit<'a, D: 'static> {
    pub(crate) store: &'a mut Option<Arc<dyn ObjectData<D>>>,
}

impl<'a, D> DataInit<'a, D> {
    pub fn init<I: Resource + 'static, U: Send + Sync + 'static>(
        &mut self,
        resource: New<I>,
        data: U,
    ) -> I
    where
        D: Dispatch<I, U> + 'static,
    {
        let arc = Arc::new(ResourceData::<I, _>::new(data));
        *self.store = Some(arc.clone() as Arc<_>);
        let mut obj = resource.id;
        obj.__set_object_data(arc);
        obj
    }

    /// Set a custom [`ObjectData`] for this object
    ///
    /// This object data is not managed by `wayland-server`, as a result you will not
    /// be able to retreive it through [`Resource::data()`](Resource::data).
    /// Instead, you'll need to directly retrieve it using
    /// [`DisplayHandle::get_object_data()`](DisplayHandle::get_object_data).
    pub fn custom_init<I: Resource + 'static>(
        &mut self,
        resource: New<I>,
        data: Arc<dyn ObjectData<D>>,
    ) -> I {
        *self.store = Some(data.clone());
        let mut obj = resource.id;
        obj.__set_object_data(data.into_any_arc());
        obj
    }
}

/*
 * Dispatch delegation helpers.
 */

impl<I, U> ResourceData<I, U> {
    pub(crate) fn new(udata: U) -> Self {
        ResourceData { marker: std::marker::PhantomData, udata }
    }
}

impl<I: Resource + 'static, U: Send + Sync + 'static, D: Dispatch<I, U> + 'static> ObjectData<D>
    for ResourceData<I, U>
{
    fn request(
        self: Arc<Self>,
        handle: &wayland_backend::server::Handle,
        data: &mut D,
        client_id: wayland_backend::server::ClientId,
        msg: wayland_backend::protocol::Message<wayland_backend::server::ObjectId>,
    ) -> Option<Arc<dyn ObjectData<D>>> {
        let dhandle = DisplayHandle::from(handle.clone());
        let client = match Client::from_id(&dhandle, client_id) {
            Ok(v) => v,
            Err(_) => {
                log::error!("Receiving a request from a dead client ?!");
                return None;
            }
        };

        let (resource, request) = match I::parse_request(&dhandle, msg) {
            Ok(v) => v,
            Err(e) => {
                log::warn!("Dispatching error encountered: {:?}, killing client.", e);
                // TODO: Kill client
                return None;
            }
        };
        let udata = resource.data::<U>().expect("Wrong user_data value for object");

        let mut new_data = None;

        <D as Dispatch<I, U>>::request(
            data,
            &client,
            &resource,
            request,
            udata,
            &dhandle,
            &mut DataInit { store: &mut new_data },
        );

        new_data
    }

    fn destroyed(
        &self,
        data: &mut D,
        client_id: wayland_backend::server::ClientId,
        object_id: wayland_backend::server::ObjectId,
    ) {
        <D as Dispatch<I, U>>::destroyed(data, client_id, object_id, &self.udata)
    }
}

/// A helper macro which delegates a set of [`Dispatch`] implementations for a resource to some other type which
/// implements [`DelegateDispatch`] for each resource.
///
/// This macro allows more easily delegating smaller parts of the protocol a compositor may wish to handle
/// in a modular fashion.
///
/// # Usage
///
/// For example, say you want to delegate events for [`WlOutput`](crate::protocol::wl_output::WlOutput)
/// to some other type.
///
/// For brevity, we will use the example in the documentation for [`DelegateDispatch`], `DelegateToMe`.
///
/// ```
/// use wayland_server::{delegate_dispatch, protocol::wl_output};
/// #
/// # use wayland_server::Dispatch;
/// #
/// # struct DelegateToMe;
/// #
/// # impl<D> Dispatch<wl_output::WlOutput, (), D> for DelegateToMe
/// # where
/// #     D: Dispatch<wl_output::WlOutput, ()> + AsMut<DelegateToMe>,
/// # {
/// #     fn request(
/// #         _state: &mut D,
/// #         _client: &wayland_server::Client,
/// #         _resource: &wl_output::WlOutput,
/// #         _request: wl_output::Request,
/// #         _data: &(),
/// #         _dhandle: &wayland_server::DisplayHandle,
/// #         _data_init: &mut wayland_server::DataInit<'_, D>,
/// #     ) {
/// #     }
/// # }
/// #
/// # type MyUserData = ();
///
/// // ExampleApp is the type events will be dispatched to.
///
/// /// The application state
/// struct ExampleApp {
///     /// The delegate for handling wl_registry events.
///     delegate: DelegateToMe,
/// }
///
/// // Use delegate_dispatch to implement Dispatch<wl_output::WlOutput, MyUserData> for ExampleApp.
/// delegate_dispatch!(ExampleApp: [wl_output::WlOutput: MyUserData] => DelegateToMe);
///
/// // DelegateToMe requires that ExampleApp implements AsMut<DelegateToMe>, so we provide the trait implementation.
/// impl AsMut<DelegateToMe> for ExampleApp {
///     fn as_mut(&mut self) -> &mut DelegateToMe {
///         &mut self.delegate
///     }
/// }
/// ```
#[macro_export]
macro_rules! delegate_dispatch {
    ($(@< $( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+ >)? $dispatch_from:ty : [$interface: ty: $udata: ty] => $dispatch_to: ty) => {
        impl$(< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $crate::Dispatch<$interface, $udata> for $dispatch_from {
            fn request(
                state: &mut Self,
                client: &$crate::Client,
                resource: &$interface,
                request: <$interface as $crate::Resource>::Request,
                data: &$udata,
                dhandle: &$crate::DisplayHandle,
                data_init: &mut $crate::DataInit<'_, Self>,
            ) {
                <$dispatch_to as $crate::Dispatch<$interface, $udata, Self>>::request(state, client, resource, request, data, dhandle, data_init)
            }

            fn destroyed(state: &mut Self, client: $crate::backend::ClientId, resource: $crate::backend::ObjectId, data: &$udata) {
                <$dispatch_to as $crate::Dispatch<$interface, $udata, Self>>::destroyed(state, client, resource, data)
            }
        }
    };
}
