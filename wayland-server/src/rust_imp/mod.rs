use std::cell::RefCell;
use std::ffi::{OsStr, OsString};
use std::io::{Error as IoError, Result as IoResult};
use std::os::unix::io::RawFd;
use std::rc::Rc;

use sources::*;
use {Implementation, Interface, NewResource, Resource};

// Clients

#[derive(Clone)]
pub(crate) struct ClientInner {}

impl ClientInner {
    pub(crate) fn alive(&self) -> bool {
        unimplemented!()
    }

    pub(crate) fn equals(&self, other: &ClientInner) -> bool {
        unimplemented!()
    }

    pub(crate) fn flush(&self) {
        unimplemented!()
    }

    pub(crate) fn kill(&self) {
        unimplemented!()
    }

    pub(crate) fn set_user_data(&self, data: *mut ()) {
        unimplemented!()
    }

    pub(crate) fn get_user_data(&self) -> *mut () {
        unimplemented!()
    }

    pub(crate) fn set_destructor(&self, destructor: fn(*mut ())) {
        unimplemented!()
    }
}

// Display

pub(crate) struct DisplayInner {}

impl DisplayInner {
    pub(crate) fn new() -> (Rc<RefCell<DisplayInner>>, EventLoopInner) {
        unimplemented!()
    }

    pub(crate) fn create_global<I: Interface, Impl>(
        &mut self,
        _: &EventLoopInner,
        version: u32,
        implementation: Impl,
    ) -> GlobalInner<I>
    where
        Impl: Implementation<NewResource<I>, u32> + 'static,
    {
        unimplemented!()
    }

    pub(crate) fn flush_clients(&mut self) {
        unimplemented!()
    }

    pub(crate) fn add_socket<S>(&mut self, name: Option<S>) -> IoResult<()>
    where
        S: AsRef<OsStr>,
    {
        unimplemented!()
    }

    pub(crate) fn add_socket_auto(&mut self) -> IoResult<OsString> {
        unimplemented!()
    }

    pub(crate) unsafe fn add_socket_fd(&mut self, fd: RawFd) -> IoResult<()> {
        unimplemented!()
    }

    pub unsafe fn create_client(&mut self, fd: RawFd) -> ClientInner {
        unimplemented!()
    }
}

// EventLoop

pub(crate) struct EventLoopInner {}

impl EventLoopInner {
    pub(crate) fn new() -> EventLoopInner {
        unimplemented!()
    }

    pub fn dispatch(&self, timeout: Option<u32>) -> IoResult<u32> {
        unimplemented!()
    }

    pub(crate) fn flush_clients_if_display(&self) {
        unimplemented!()
    }

    pub fn add_fd_event_source<Impl>(
        &self,
        fd: RawFd,
        interest: FdInterest,
        implementation: Impl,
    ) -> Result<SourceInner<FdEvent>, (IoError, Impl)>
    where
        Impl: Implementation<(), FdEvent> + 'static,
    {
        unimplemented!()
    }

    pub fn add_timer_event_source<Impl>(
        &self,
        implementation: Impl,
    ) -> Result<SourceInner<TimerEvent>, (IoError, Impl)>
    where
        Impl: Implementation<(), TimerEvent> + 'static,
    {
        unimplemented!()
    }

    pub fn add_signal_event_source<Impl>(
        &self,
        signal: ::nix::sys::signal::Signal,
        implementation: Impl,
    ) -> Result<SourceInner<SignalEvent>, (IoError, Impl)>
    where
        Impl: Implementation<(), SignalEvent> + 'static,
    {
        unimplemented!()
    }

    pub fn add_idle_event_source<Impl>(&self, implementation: Impl) -> IdleSourceInner
    where
        Impl: Implementation<(), ()> + 'static,
    {
        unimplemented!()
    }
}

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

// Sources

pub(crate) struct SourceInner<E> {
    _e: ::std::marker::PhantomData<*const E>,
}

impl<E> SourceInner<E> {
    pub(crate) fn remove(self) -> Box<Implementation<(), E>> {
        unimplemented!()
    }
}

// FD event source

impl SourceInner<FdEvent> {
    pub(crate) fn update_mask(&mut self, mask: FdInterest) {
        unimplemented!()
    }
}

// Timer event source

impl SourceInner<TimerEvent> {
    pub(crate) fn set_delay_ms(&mut self, delay: i32) {
        unimplemented!()
    }
}

// Idle event source

pub(crate) struct IdleSourceInner {}

impl IdleSourceInner {
    pub(crate) fn remove(self) -> Box<Implementation<(), ()>> {
        unimplemented!()
    }
}
