use libc::{c_int, c_void, uint32_t};

use ffi::abi::wl_proxy;
#[cfg(not(feature = "dlopen"))]
use ffi::abi::{wl_proxy_destroy, wl_proxy_add_listener, wl_proxy_set_user_data,
               wl_proxy_get_user_data};
#[cfg(feature = "dlopen")]
use ffi::abi::WAYLAND_CLIENT_HANDLE as WCH;

#[repr(C)] pub struct wl_callback;

#[repr(C)]
pub struct wl_callback_listener {
    pub done: extern fn(data: *mut c_void, callback: *mut wl_callback, data: uint32_t)
}

#[inline(always)]
pub unsafe fn wl_callback_add_listener(callback: *mut wl_callback,
                                       listener: *const wl_callback_listener,
                                       data: *mut c_void
                                      ) -> c_int {
    ffi_dispatch!(WCH, wl_proxy_add_listener,
        callback as *mut wl_proxy,
        listener as *mut extern fn(),
        data
    )
}

#[inline(always)]
pub unsafe fn wl_callback_set_user_data(callback: *mut wl_callback, user_data: *mut c_void) {
    ffi_dispatch!(WCH, wl_proxy_set_user_data, callback as *mut wl_proxy, user_data)
}

#[inline(always)]
pub unsafe fn wl_callback_get_user_data(callback: *mut wl_callback) -> *mut c_void {
    ffi_dispatch!(WCH, wl_proxy_get_user_data, callback as *mut wl_proxy)
}

#[inline(always)]
pub unsafe fn wl_callback_destroy(callback: *mut wl_callback) {
    ffi_dispatch!(WCH, wl_proxy_destroy, callback as *mut wl_proxy)
}