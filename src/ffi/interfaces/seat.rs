use std::ptr;

use libc::{c_int, c_void, c_char, uint32_t};

use ffi::abi::wl_proxy;
use ffi::abi::WAYLAND_CLIENT_HANDLE as WCH;
use ffi::enums::wl_seat_capability;

use super::keyboard::wl_keyboard;
use super::pointer::wl_pointer;
use super::touch::wl_touch;

#[repr(C)] pub struct wl_seat;

#[repr(C)]
pub struct wl_seat_listener {
    pub capabilities: extern fn(data: *mut c_void,
                                seat: *mut wl_seat,
                                capabilities: wl_seat_capability
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
    (WCH.wl_proxy_add_listener)(
        seat as *mut wl_proxy,
        listener as *mut extern fn(),
        data
    )
}

#[inline(always)]
pub unsafe fn wl_seat_set_user_data(seat: *mut wl_seat, data: *mut c_void) {
    (WCH.wl_proxy_set_user_data)(seat as *mut wl_proxy, data)
}

#[inline(always)]
pub unsafe fn wl_seat_get_user_data(seat: *mut wl_seat) -> *mut c_void {
    (WCH.wl_proxy_get_user_data)(seat as *mut wl_proxy)
}

#[inline(always)]
pub unsafe fn wl_seat_destroy(seat: *mut wl_seat) {
    (WCH.wl_proxy_destroy)(seat as *mut wl_proxy)
}

#[inline(always)]
pub unsafe fn wl_seat_get_pointer(seat: *mut wl_seat) -> *mut wl_pointer {
    (WCH.wl_proxy_marshal_constructor)(
        seat as *mut wl_proxy,
        WL_SEAT_GET_POINTER,
        WCH.wl_pointer_interface,
        ptr::null_mut::<c_void>()
    ) as *mut wl_pointer
}

#[inline(always)]
pub unsafe fn wl_seat_get_keyboard(seat: *mut wl_seat) -> *mut wl_keyboard {
    (WCH.wl_proxy_marshal_constructor)(
        seat as *mut wl_proxy,
        WL_SEAT_GET_KEYBOARD,
        WCH.wl_keyboard_interface,
        ptr::null_mut::<c_void>()
    ) as *mut wl_keyboard
}

#[inline(always)]
pub unsafe fn wl_seat_get_touch(seat: *mut wl_seat) -> *mut wl_touch {
    (WCH.wl_proxy_marshal_constructor)(
        seat as *mut wl_proxy,
        WL_SEAT_GET_TOUCH,
        WCH.wl_touch_interface,
        ptr::null_mut::<c_void>()
    ) as *mut wl_touch
}