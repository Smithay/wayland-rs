use libc::{c_int, c_void, uint32_t, int32_t};

use ffi::abi::{wl_proxy, wl_fixed_t};
#[cfg(not(feature = "dlopen"))]
use ffi::abi::{wl_proxy_destroy, wl_proxy_add_listener, wl_proxy_set_user_data,
               wl_proxy_get_user_data, wl_proxy_marshal};
#[cfg(feature = "dlopen")]
use ffi::abi::WAYLAND_CLIENT_HANDLE as WCH;

use super::surface::wl_surface;

pub enum wl_touch { }

#[repr(C)]
pub struct wl_touch_listener {
    pub down: extern fn(data: *mut c_void,
                        touch: *mut wl_touch,
                        serial: uint32_t,
                        time: uint32_t,
                        surface: *mut wl_surface,
                        id: int32_t,
                        x: wl_fixed_t,
                        y: wl_fixed_t
                       ),
    pub up: extern fn(data: *mut c_void,
                      touch: *mut wl_touch,
                      serial: uint32_t,
                      time: uint32_t,
                      id: int32_t
                     ),
    pub motion: extern fn(data: *mut c_void,
                          touch: *mut wl_touch,
                          time: uint32_t,
                          id: int32_t,
                          x: wl_fixed_t,
                          y: wl_fixed_t
                         ),
    pub frame: extern fn(data: *mut c_void, touch: *mut wl_touch),
    pub cancel: extern fn(data: *mut c_void, touch: *mut wl_touch)
}

const WL_TOUCH_RELEASE: uint32_t = 0;

#[inline(always)]
pub unsafe fn wl_touch_add_listener(touch: *mut wl_touch,
                                    listener: *const wl_touch_listener,
                                    data: *mut c_void
                                   ) -> c_int {
    ffi_dispatch!(WCH, wl_proxy_add_listener,
        touch as *mut wl_proxy,
        listener as *mut extern fn(),
        data
    )
}

#[inline(always)]
pub unsafe fn wl_touch_set_user_data(touch: *mut wl_touch, data: *mut c_void) {
    ffi_dispatch!(WCH, wl_proxy_set_user_data,touch as *mut wl_proxy, data)
}

#[inline(always)]
pub unsafe fn wl_touch_get_user_data(touch: *mut wl_touch) -> *mut c_void {
    ffi_dispatch!(WCH, wl_proxy_get_user_data,touch as *mut wl_proxy)
}

#[inline(always)]
pub unsafe fn wl_touch_destroy(touch: *mut wl_touch) {
    ffi_dispatch!(WCH, wl_proxy_destroy,touch as *mut wl_proxy)
}

#[inline(always)]
pub unsafe fn wl_touch_release(touch: *mut wl_touch) {
    ffi_dispatch!(WCH, wl_proxy_marshal,touch as *mut wl_proxy, WL_TOUCH_RELEASE);
    ffi_dispatch!(WCH, wl_proxy_destroy,touch as *mut wl_proxy)
}