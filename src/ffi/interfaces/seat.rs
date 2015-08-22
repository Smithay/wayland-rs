use std::ptr;

use libc::{c_int, c_void, c_char, uint32_t};

use ffi::abi::wl_proxy;
#[cfg(not(feature = "dlopen"))]
use ffi::abi::{wl_proxy_destroy, wl_proxy_add_listener, wl_proxy_set_user_data,
               wl_proxy_get_user_data, wl_proxy_marshal_constructor,
               wl_pointer_interface, wl_keyboard_interface, wl_touch_interface};
#[cfg(feature = "dlopen")]
use ffi::abi::WAYLAND_CLIENT_HANDLE as WCH;
use ffi::enums::SeatCapability;

use super::keyboard::wl_keyboard;
use super::pointer::wl_pointer;
use super::touch::wl_touch;

pub enum wl_seat { }

#[repr(C)]
pub struct wl_seat_listener {
    pub capabilities: extern fn(data: *mut c_void,
                                seat: *mut wl_seat,
                                capabilities: SeatCapability
                               ),
    pub name: extern fn(data: *mut c_void,
                        seat: *mut wl_seat,
                        name: *const c_char
                       )
}

const WL_SEAT_GET_POINTER: uint32_t = 0;
const WL_SEAT_GET_KEYBOARD: uint32_t = 1;
const WL_SEAT_GET_TOUCH: uint32_t = 2;

#[inline(always)]
pub unsafe fn wl_seat_add_listener(seat: *mut wl_seat,
                                   listener: *const wl_seat_listener,
                                   data: *mut c_void
                                  ) -> c_int {
    ffi_dispatch!(WCH, wl_proxy_add_listener,
        seat as *mut wl_proxy,
        listener as *mut extern fn(),
        data
    )
}

#[inline(always)]
pub unsafe fn wl_seat_set_user_data(seat: *mut wl_seat, data: *mut c_void) {
    ffi_dispatch!(WCH, wl_proxy_set_user_data,seat as *mut wl_proxy, data)
}

#[inline(always)]
pub unsafe fn wl_seat_get_user_data(seat: *mut wl_seat) -> *mut c_void {
    ffi_dispatch!(WCH, wl_proxy_get_user_data,seat as *mut wl_proxy)
}

#[inline(always)]
pub unsafe fn wl_seat_destroy(seat: *mut wl_seat) {
    ffi_dispatch!(WCH, wl_proxy_destroy,seat as *mut wl_proxy)
}

#[inline(always)]
pub unsafe fn wl_seat_get_pointer(seat: *mut wl_seat) -> *mut wl_pointer {
    ffi_dispatch!(WCH, wl_proxy_marshal_constructor,
        seat as *mut wl_proxy,
        WL_SEAT_GET_POINTER,
        ffi_dispatch_static!(WCH, wl_pointer_interface),
        ptr::null_mut::<c_void>()
    ) as *mut wl_pointer
}

#[inline(always)]
pub unsafe fn wl_seat_get_keyboard(seat: *mut wl_seat) -> *mut wl_keyboard {
    ffi_dispatch!(WCH, wl_proxy_marshal_constructor,
        seat as *mut wl_proxy,
        WL_SEAT_GET_KEYBOARD,
        ffi_dispatch_static!(WCH, wl_keyboard_interface),
        ptr::null_mut::<c_void>()
    ) as *mut wl_keyboard
}

#[inline(always)]
pub unsafe fn wl_seat_get_touch(seat: *mut wl_seat) -> *mut wl_touch {
    ffi_dispatch!(WCH, wl_proxy_marshal_constructor,
        seat as *mut wl_proxy,
        WL_SEAT_GET_TOUCH,
        ffi_dispatch_static!(WCH, wl_touch_interface),
        ptr::null_mut::<c_void>()
    ) as *mut wl_touch
}