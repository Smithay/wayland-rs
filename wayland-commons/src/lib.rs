#[macro_use]
extern crate downcast_rs as downcast;
#[cfg(feature = "native_lib")]
extern crate wayland_sys;
#[cfg(feature = "native_lib")]
use wayland_sys::common as syscom;

use std::os::raw::c_void;

use downcast::Downcast;

pub trait MessageGroup: Sized {
    fn is_destructor(&self) -> bool;
    #[cfg(feature = "native_lib")]
    unsafe fn from_raw_c(obj: *mut c_void, opcode: u32, args: *const syscom::wl_argument)
        -> Result<Self, ()>;
    #[cfg(feature = "native_lib")]
    fn as_raw_c_in<F, T>(self, f: F) -> T
    where
        F: FnOnce(u32, &mut [syscom::wl_argument]) -> T;
}

pub trait Interface: 'static {
    type Requests: MessageGroup + 'static;
    type Events: MessageGroup + 'static;

    const NAME: &'static str;

    #[cfg(feature = "native_lib")]
    fn c_interface() -> *const ::syscom::wl_interface;
}

pub trait Implementation<Meta, Msg>: Downcast {
    fn receive(&mut self, msg: Msg, meta: Meta);
}

impl_downcast!(Implementation<Meta, Msg>);

impl<Meta, Msg, F> Implementation<Meta, Msg> for F
where
    F: FnMut(Msg, Meta) + 'static,
{
    fn receive(&mut self, msg: Msg, meta: Meta) {
        (self)(msg, meta)
    }
}

pub fn downcast_impl<Msg: 'static, Meta: 'static, T: Implementation<Meta, Msg>>(
    b: Box<Implementation<Meta, Msg>>,
) -> Result<Box<T>, Box<Implementation<Meta, Msg>>> {
    if b.is::<T>() {
        unsafe {
            let raw: *mut Implementation<Meta, Msg> = Box::into_raw(b);
            Ok(Box::from_raw(raw as *mut T))
        }
    } else {
        Err(b)
    }
}

pub struct AnonymousObject;

pub enum NoMessage {
}

impl Interface for AnonymousObject {
    type Requests = NoMessage;
    type Events = NoMessage;
    const NAME: &'static str = "";
    #[cfg(feature = "native_lib")]
    fn c_interface() -> *const ::syscom::wl_interface {
        ::std::ptr::null()
    }
}

impl MessageGroup for NoMessage {
    fn is_destructor(&self) -> bool {
        match *self {}
    }
    unsafe fn from_raw_c(
        _obj: *mut c_void,
        _opcode: u32,
        _args: *const syscom::wl_argument,
    ) -> Result<Self, ()> {
        Err(())
    }
    fn as_raw_c_in<F, T>(self, _f: F) -> T
    where
        F: FnOnce(u32, &mut [syscom::wl_argument]) -> T,
    {
        match self {}
    }
}
