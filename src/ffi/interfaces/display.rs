use std::ptr;

use libc::{c_int, c_void,c_char, uint32_t};

use ffi::abi::wl_proxy;
use ffi::abi::WAYLAND_CLIENT_HANDLE as WCH;

pub use ffi::abi::wl_display;

use super::callback::wl_callback;
use super::registry::wl_registry;

#[repr(C)]
pub struct wl_display_listener {
    pub error: extern fn(data: *mut c_void,
                         display: *mut wl_display,
                         object_id: *mut c_void,
                         code: uint32_t,
                         message: *const c_char
                        ),
    pub delete_id: extern fn(data: *mut c_void,
                             display: *mut wl_display,
                             id: uint32_t
                            )
}

const WL_DISPLAY_SYNC: uint32_t = 0;
const WL_DISPLAY_GET_REGISTRY: uint32_t = 1;

#[inline(always)]
pub unsafe fn wl_display_add_listener(display: *mut wl_display,
                                      listener: *const wl_display_listener,
                                      data: *mut c_void
                                     ) -> c_int {
    (WCH.wl_proxy_add_listener)(
        display as *mut wl_proxy,
        listener as *mut extern fn(),
        data
    )
}

#[inline(always)]
pub unsafe fn wl_display_set_user_data(display: *mut wl_display, data: *mut c_void) {
    (WCH.wl_proxy_set_user_data)(display as *mut wl_proxy, data)
}

#[inline(always)]
pub unsafe fn wl_display_get_user_data(display: *mut wl_display) -> *mut c_void {
    (WCH.wl_proxy_get_user_data)(display as *mut wl_proxy)
}

#[inline(always)]
pub unsafe fn wl_display_sync(display: *mut wl_display) -> *mut wl_callback {
    (WCH.wl_proxy_marshal_constructor)(
        display as *mut wl_proxy,
        WL_DISPLAY_SYNC,
        WCH.wl_callback_interface,
        ptr::null_mut::<c_void>()
    ) as *mut wl_callback
}

#[inline(always)]
pub unsafe fn wl_display_get_registry(display: *mut wl_display) -> *mut wl_registry {
    (WCH.wl_proxy_marshal_constructor)(
        display as *mut wl_proxy,
        WL_DISPLAY_GET_REGISTRY,
        WCH.wl_registry_interface,
        ptr::null_mut::<c_void>()
    ) as *mut wl_registry
}