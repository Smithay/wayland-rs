#[cfg(feature = "native_lib")]
extern crate wayland_sys;
#[cfg(feature = "native_lib")]
use wayland_sys::common as syscom;

use std::os::raw::c_void;

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

pub type Implementation<Meta, M, ID> = fn(Meta, M, &mut ID);

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
        obj: *mut c_void,
        opcode: u32,
        args: *const syscom::wl_argument,
    ) -> Result<Self, ()> {
        Err(())
    }
    fn as_raw_c_in<F, T>(self, f: F) -> T
    where
        F: FnOnce(u32, &mut [syscom::wl_argument]) -> T,
    {
        match self {}
    }
}
