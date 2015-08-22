use libc::{c_int, c_void, uint32_t, int32_t};

use ffi::abi::{wl_proxy, wl_fixed_t};
#[cfg(not(feature = "dlopen"))]
use ffi::abi::{wl_proxy_destroy, wl_proxy_add_listener, wl_proxy_set_user_data,
               wl_proxy_get_user_data, wl_proxy_marshal};
#[cfg(feature = "dlopen")]
use ffi::abi::WAYLAND_CLIENT_HANDLE as WCH;
use ffi::enums::{ButtonState, ScrollAxis};

use super::surface::wl_surface;

pub enum wl_pointer { }

#[repr(C)]
pub struct wl_pointer_listener {
    pub enter: extern fn(data: *mut c_void,
                         pointer: *mut wl_pointer,
                         serial: uint32_t,
                         surface: *mut wl_surface,
                         surface_x: wl_fixed_t,
                         surface_y: wl_fixed_t
                        ),
    pub leave: extern fn(data: *mut c_void,
                         pointer: *mut wl_pointer,
                         serial: uint32_t,
                         surface: *mut wl_surface
                        ),
    pub motion: extern fn(data: *mut c_void,
                          pointer: *mut wl_pointer,
                          time: uint32_t,
                          surface_x: wl_fixed_t,
                          surface_y: wl_fixed_t
                         ),
    pub button: extern fn(data: *mut c_void,
                          pointer: *mut wl_pointer,
                          serial: uint32_t,
                          time: uint32_t,
                          button: uint32_t,
                          state: ButtonState
                         ),
    pub axis: extern fn(data: *mut c_void,
                        pointer: *mut wl_pointer,
                        time: uint32_t,
                        axis: ScrollAxis,
                        value: wl_fixed_t
                       )
}

const WL_POINTER_SET_CURSOR: uint32_t = 0;
const WL_POINTER_RELEASE: uint32_t = 1;

#[inline(always)]
pub unsafe fn wl_pointer_add_listener(pointer: *mut wl_pointer,
                                      listener: *const wl_pointer_listener,
                                      data: *mut c_void
                                     ) -> c_int {
    ffi_dispatch!(WCH, wl_proxy_add_listener,
        pointer as *mut wl_proxy,
        listener as *mut extern fn(),
        data
    )
}

#[inline(always)]
pub unsafe fn wl_pointer_set_user_data(pointer: *mut wl_pointer, data: *mut c_void) {
    ffi_dispatch!(WCH, wl_proxy_set_user_data,pointer as *mut wl_proxy, data)
}

#[inline(always)]
pub unsafe fn wl_pointer_get_user_data(pointer: *mut wl_pointer) -> *mut c_void {
    ffi_dispatch!(WCH, wl_proxy_get_user_data,pointer as *mut wl_proxy)
}

#[inline(always)]
pub unsafe fn wl_pointer_destroy(pointer: *mut wl_pointer) {
    ffi_dispatch!(WCH, wl_proxy_destroy,pointer as *mut wl_proxy)
}

#[inline(always)]
pub unsafe fn wl_pointer_set_cursor(pointer: *mut wl_pointer,
                                    serial: uint32_t,
                                    surface: *mut wl_surface,
                                    hotspot_x: int32_t,
                                    hotspot_y: int32_t
                                   ) {
    ffi_dispatch!(WCH, wl_proxy_marshal,
        pointer as *mut wl_proxy,
        WL_POINTER_SET_CURSOR,
        serial,
        surface,
        hotspot_x,
        hotspot_y
    )
}

#[inline(always)]
pub unsafe fn wl_pointer_release(pointer: *mut wl_pointer) {
    ffi_dispatch!(WCH, wl_proxy_marshal,pointer as *mut wl_proxy, WL_POINTER_RELEASE);
    ffi_dispatch!(WCH, wl_proxy_destroy,pointer as *mut wl_proxy)
}