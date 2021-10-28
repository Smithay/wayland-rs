use std::sync::Arc;

use wayland_backend::server::{
    ClientData, ClientId, GlobalHandler, GlobalId, Handle, ObjectData, ObjectId,
};

use crate::{dispatch::ResourceData, Client, Dispatch, DisplayHandle, Resource};

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

        let udata = data.bind(&mut handle, &client, &resource, &self.data);

        Arc::new(ResourceData::<I, _>::new(udata))
    }
}

pub trait GlobalDispatch<I: Resource>: Dispatch<I> {
    type GlobalData: Send + Sync + 'static;

    fn bind(
        &mut self,
        handle: &mut DisplayHandle<'_, Self>,
        client: &Client,
        resource: &I,
        global_data: &Self::GlobalData,
    ) -> <Self as Dispatch<I>>::UserData;

    fn can_view(_client: Client, _global_data: &Self::GlobalData) -> bool {
        true
    }
}
