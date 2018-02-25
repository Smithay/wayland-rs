#[cfg(feature = "native_lib")]
extern crate wayland_sys;
#[cfg(feature = "native_lib")]
use wayland_sys::common as syscom;

pub trait MessageGroup: Sized {
    fn is_destructor(&self) -> bool;
    #[cfg(feature = "native_lib")]
    unsafe fn from_raw_c(opcode: u32, args: *const syscom::wl_argument) -> Result<Self, ()>;
    #[cfg(feature = "native_lib")]
    fn as_raw_c_in<F, T>(self, f: F) -> T where F: FnOnce(u32, &[syscom::wl_argument]) -> T;
}

pub trait Interface: 'static {
    type Requests: MessageGroup + 'static;
    type Events: MessageGroup + 'static;

    #[cfg(feature = "native_lib")]
    const C_INTERFACE: *const ::syscom::wl_interface;

    fn name() -> &'static str;
}

pub type Implementation<Meta, M, ID> = fn(Meta, M, &mut ID);

