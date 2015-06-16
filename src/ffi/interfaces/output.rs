use libc::{c_int, c_void, c_char, int32_t};

use ffi::abi::wl_proxy;
#[cfg(not(feature = "dlopen"))]
use ffi::abi::{wl_proxy_destroy, wl_proxy_add_listener, wl_proxy_set_user_data,
               wl_proxy_get_user_data};
#[cfg(feature = "dlopen")]
use ffi::abi::WAYLAND_CLIENT_HANDLE as WCH;
use ffi::enums::{OutputMode, OutputSubpixel, OutputTransform};

#[repr(C)] pub struct wl_output;

#[repr(C)]
pub struct wl_output_listener {
    pub geometry: extern fn(data: *mut c_void,
                            output: *mut wl_output,
                            x: int32_t,
                            y: int32_t,
                            physical_width: int32_t,
                            physical_height: int32_t,
                            subpixel: OutputSubpixel,
                            make: *const c_char,
                            model: *const c_char,
                            transform: OutputTransform
                           ),
    pub mode: extern fn(data: *mut c_void,
                        output: *mut wl_output,
                        flags: OutputMode,
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
    ffi_dispatch!(WCH, wl_proxy_add_listener,
        output as *mut wl_proxy,
        listener as *mut extern fn(),
        data
    )
}

#[inline(always)]
pub unsafe fn wl_output_set_user_data(output: *mut wl_output, data: *mut c_void) {
    ffi_dispatch!(WCH, wl_proxy_set_user_data,output as *mut wl_proxy, data)
}

#[inline(always)]
pub unsafe fn wl_output_get_user_data(output: *mut wl_output) -> *mut c_void {
    ffi_dispatch!(WCH, wl_proxy_get_user_data,output as *mut wl_proxy)
}

#[inline(always)]
pub unsafe fn wl_output_destroy(output: *mut wl_output) {
    ffi_dispatch!(WCH, wl_proxy_destroy,output as *mut wl_proxy)
}