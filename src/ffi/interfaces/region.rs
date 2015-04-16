use libc::{c_void, uint32_t, int32_t};

use ffi::abi::{self, wl_proxy};

#[repr(C)] pub struct wl_region;

const WL_REGION_DESTROY: uint32_t = 0;
const WL_REGION_ADD: uint32_t = 1;
const WL_REGION_SUBTRACT: uint32_t = 2;

#[inline(always)]
pub unsafe fn wl_region_set_user_data(region: *mut wl_region, data: *mut c_void) {
    abi::wl_proxy_set_user_data(region as *mut wl_proxy, data)
}

#[inline(always)]
pub unsafe fn wl_region_get_user_data(region: *mut wl_region) -> *mut c_void {
    abi::wl_proxy_get_user_data(region as *mut wl_proxy)
}

#[inline(always)]
pub unsafe fn wl_region_destroy(region: *mut wl_region) {
    abi::wl_proxy_marshal(region as *mut wl_proxy, WL_REGION_DESTROY);
    abi::wl_proxy_destroy(region as *mut wl_proxy)
}

#[inline(always)]
pub unsafe fn wl_region_add(region: *mut wl_region,
                            x: int32_t,
                            y: int32_t,
                            width: int32_t,
                            height: int32_t
                           ) {
    abi::wl_proxy_marshal(region as *mut wl_proxy, WL_REGION_ADD, x, y, width, height)
}

#[inline(always)]
pub unsafe fn wl_region_subtract(region: *mut wl_region,
                                 x: int32_t,
                                 y: int32_t,
                                 width: int32_t,
                                 height: int32_t
                                ) {
    abi::wl_proxy_marshal(region as *mut wl_proxy, WL_REGION_SUBTRACT, x, y, width, height)
}