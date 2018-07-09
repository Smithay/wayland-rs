use {Implementation, Interface, Resource};

mod clients;
mod display;
mod event_loop;

pub(crate) use self::clients::ClientInner;
pub(crate) use self::display::DisplayInner;
pub(crate) use self::event_loop::{EventLoopInner, IdleSourceInner, SourceInner};

// Globals

pub(crate) struct GlobalInner<I: Interface> {
    _i: ::std::marker::PhantomData<*const I>,
}

impl<I: Interface> GlobalInner<I> {
    pub fn destroy(self) {
        unimplemented!()
    }
}

// Resource

pub(crate) struct ResourceInner {}

impl ResourceInner {
    pub(crate) fn send<I: Interface>(&self, msg: I::Event) {
        unimplemented!()
    }

    pub(crate) fn is_alive(&self) -> bool {
        unimplemented!()
    }

    pub(crate) fn version(&self) -> u32 {
        unimplemented!()
    }

    pub(crate) fn equals(&self, other: &ResourceInner) -> bool {
        unimplemented!()
    }

    pub(crate) fn same_client_as(&self, other: &ResourceInner) -> bool {
        unimplemented!()
    }

    pub(crate) fn post_error(&self, error_code: u32, msg: String) {
        unimplemented!()
    }

    pub(crate) fn set_user_data(&self, ptr: *mut ()) {
        unimplemented!()
    }

    pub(crate) fn get_user_data(&self) -> *mut () {
        unimplemented!()
    }

    pub(crate) fn client(&self) -> Option<ClientInner> {
        unimplemented!()
    }

    pub(crate) fn id(&self) -> u32 {
        unimplemented!()
    }

    pub(crate) fn is_implemented_with<I: Interface, Impl>(&self) -> bool
    where
        Impl: Implementation<Resource<I>, I::Request> + 'static,
    {
        unimplemented!()
    }

    pub(crate) fn clone(&self) -> ResourceInner {
        unimplemented!()
    }
}

pub(crate) struct NewResourceInner {}

impl NewResourceInner {
    pub(crate) unsafe fn implement<I: Interface, Impl, Dest>(
        self,
        implementation: Impl,
        destructor: Option<Dest>,
        token: Option<&EventLoopInner>,
    ) -> ResourceInner
    where
        Impl: Implementation<Resource<I>, I::Request> + 'static,
        Dest: FnMut(Resource<I>, Box<Implementation<Resource<I>, I::Request>>) + 'static,
    {
        unimplemented!()
    }
}
