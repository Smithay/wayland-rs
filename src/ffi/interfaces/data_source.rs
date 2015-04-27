use libc::{c_int, c_void, c_char, uint32_t, int32_t};

use ffi::abi::wl_proxy;
use ffi::abi::WAYLAND_CLIENT_HANDLE as WCH;

#[repr(C)] pub struct wl_data_source;

#[repr(C)]
pub struct wl_data_source_listener {
    pub target: extern fn(data: *mut c_void,
                          data_source: *mut wl_data_source,
                          mime_type: *const c_char
                         ),
    pub send: extern fn(data: *mut c_void,
                        data_source: *mut wl_data_source,
                        mime_type: *const c_char,
                        fd: int32_t
                       ),
    pub cancelled: extern fn(data: *mut c_void,
                             data_source: *mut wl_data_source
                            )
}

const WL_DATA_SOURCE_OFFER: uint32_t = 0;
const WL_DATA_SOURCE_DESTROY: uint32_t = 1;

#[inline(always)]
pub unsafe fn wl_data_source_add_listener(data_source: *mut wl_data_source,
                                          listener: *const wl_data_source_listener,
                                          data: *mut c_void
                                         ) -> c_int {
    (WCH.wl_proxy_add_listener)(
        data_source as *mut wl_proxy,
        listener as *mut extern fn(),
        data
    )
}

#[inline(always)]
pub unsafe fn wl_data_source_set_user_data(data_source: *mut wl_data_source, data: *mut c_void) {
    (WCH.wl_proxy_set_user_data)(data_source as *mut wl_proxy, data)
}

#[inline(always)]
pub unsafe fn wl_data_source_get_user_data(data_source: *mut wl_data_source) -> *mut c_void {
    (WCH.wl_proxy_get_user_data)(data_source as *mut wl_proxy)
}

#[inline(always)]
pub unsafe fn wl_data_source_offer(data_source: *mut wl_data_source, mime_type: *const c_char) {
    (WCH.wl_proxy_marshal)(
        data_source as *mut wl_proxy,
        WL_DATA_SOURCE_OFFER,
        mime_type
    )
}

#[inline(always)]
pub unsafe fn wl_data_source_destroy(data_source: *mut wl_data_source) {
    (WCH.wl_proxy_marshal)(data_source as *mut wl_proxy, WL_DATA_SOURCE_DESTROY);
    (WCH.wl_proxy_destroy)(data_source as *mut wl_proxy)
}