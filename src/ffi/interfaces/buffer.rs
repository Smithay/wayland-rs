use libc::{c_int, c_void, uint32_t};

use ffi::abi::wl_proxy;
use ffi::abi::WAYLAND_CLIENT_HANDLE as WCH;

#[repr(C)] pub struct wl_buffer;

#[repr(C)]
pub struct wl_buffer_listener {
    pub release: extern fn(data: *mut c_void, wl_buffer: *mut wl_buffer)
}

const WL_BUFFER_DESTROY: uint32_t = 0;

#[inline(always)]
pub unsafe fn wl_buffer_add_listener(buffer: *mut wl_buffer,
                                     listener: *const wl_buffer_listener,
                                     data: *mut c_void
                                    ) -> c_int {
    (WCH.wl_proxy_add_listener)(
        buffer as *mut wl_proxy,
        listener as *mut extern fn(),
        data
    )
}

#[inline(always)]
pub unsafe fn wl_buffer_set_user_data(buffer: *mut wl_buffer, data: *mut c_void) {
    (WCH.wl_proxy_set_user_data)(buffer as *mut wl_proxy, data)
}

#[inline(always)]
pub unsafe fn wl_buffer_get_user_data(buffer: *mut wl_buffer) -> *mut c_void {
    (WCH.wl_proxy_get_user_data)(buffer as *mut wl_proxy)
}

#[inline(always)]
pub unsafe fn wl_buffer_destroy(buffer: *mut wl_buffer) {
    (WCH.wl_proxy_marshal)(buffer as *mut wl_proxy, WL_BUFFER_DESTROY);
    (WCH.wl_proxy_destroy)(buffer as *mut wl_proxy)
}