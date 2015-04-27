use libc::{c_int, c_void, c_char, uint32_t, int32_t};

use ffi::abi::wl_proxy;
use ffi::abi::WAYLAND_CLIENT_HANDLE as WCH;

#[repr(C)] pub struct wl_output;

#[repr(C)]
pub struct wl_output_listener {
    pub geometry: extern fn(data: *mut c_void,
                            output: *mut wl_output,
                            x: int32_t,
                            y: int32_t,
                            physical_width: int32_t,
                            physical_height: int32_t,
                            subpixel: int32_t,
                            make: *const c_char,
                            model: *const c_char,
                            transform: int32_t
                           ),
    pub mode: extern fn(data: *mut c_void,
                        output: *mut wl_output,
                        flags: uint32_t,
                        width: int32_t,
                        height: int32_t,
                        refresh: int32_t
                       ),
    pub done: extern fn(data: *mut c_void, output: *mut wl_output),
    pub scale: extern fn(data: *mut c_void,
                         output: *mut wl_output,
                         factor: int32_t
                        )
}

#[inline(always)]
pub unsafe fn wl_output_add_listener(output: *mut wl_output,
                                     listener: *const wl_output_listener,
                                     data: *mut c_void
                                    ) -> c_int {
    (WCH.wl_proxy_add_listener)(
        output as *mut wl_proxy,
        listener as *mut extern fn(),
        data
    )
}

#[inline(always)]
pub unsafe fn wl_output_set_user_data(output: *mut wl_output, data: *mut c_void) {
    (WCH.wl_proxy_set_user_data)(output as *mut wl_proxy, data)
}

#[inline(always)]
pub unsafe fn wl_output_get_user_data(output: *mut wl_output) -> *mut c_void {
    (WCH.wl_proxy_get_user_data)(output as *mut wl_proxy)
}

#[inline(always)]
pub unsafe fn wl_output_destroy(output: *mut wl_output) {
    (WCH.wl_proxy_destroy)(output as *mut wl_proxy)
}