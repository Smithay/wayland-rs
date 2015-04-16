use std::ptr;

use libc::{c_void, uint32_t};

use ffi::abi::{self, wl_proxy};

use super::region::wl_region;
use super::surface::wl_surface;

#[repr(C)] pub struct wl_compositor;

const WL_COMPOSITOR_CREATE_SURFACE: uint32_t = 0;
const WL_COMPOSITOR_CREATE_REGION: uint32_t = 1;

#[inline(always)]
pub unsafe fn wl_compositor_set_user_data(compositor: *mut wl_compositor, data: *mut c_void) {
    abi::wl_proxy_set_user_data(compositor as *mut wl_proxy, data)
}

#[inline(always)]
pub unsafe fn wl_compositor_get_user_data(compositor: *mut wl_compositor) -> *mut c_void {
    abi::wl_proxy_get_user_data(compositor as *mut wl_proxy)
}

#[inline(always)]
pub unsafe fn wl_compositor_destroy(compositor: *mut wl_compositor) {
    abi::wl_proxy_destroy(compositor as *mut wl_proxy)
}

#[inline(always)]
pub unsafe fn wl_compositor_create_surface(compositor: *mut wl_compositor) -> *mut wl_surface {
    abi::wl_proxy_marshal_constructor(
        compositor as *mut wl_proxy,
        WL_COMPOSITOR_CREATE_SURFACE,
        &abi::wl_surface_interface,
        ptr::null_mut::<c_void>()
    ) as *mut wl_surface
}

#[inline(always)]
pub unsafe fn wl_compositor_create_region(compositor: *mut wl_compositor) -> *mut wl_region {
    abi::wl_proxy_marshal_constructor(
        compositor as *mut wl_proxy,
        WL_COMPOSITOR_CREATE_REGION,
        &abi::wl_region_interface,
        ptr::null_mut::<c_void>()
    ) as *mut wl_region
}