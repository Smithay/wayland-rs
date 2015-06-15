use std::ptr;

use libc::{c_void, uint32_t};

use ffi::abi::wl_proxy;
#[cfg(not(feature = "dlopen"))]
use ffi::abi::{wl_proxy_destroy, wl_proxy_set_user_data, wl_proxy_get_user_data,
               wl_proxy_marshal, wl_proxy_marshal_constructor, wl_subsurface_interface};
#[cfg(feature = "dlopen")]
use ffi::abi::WAYLAND_CLIENT_HANDLE as WCH;

use super::subsurface::wl_subsurface;
use super::surface::wl_surface;

#[repr(C)] pub struct wl_subcompositor;

const WL_SUBCOMPOSITOR_DESTROY: uint32_t = 0;
const WL_SUBCOMPOSITOR_GET_SUBSURFACE: uint32_t = 1;

#[inline(always)]
pub unsafe fn wl_subcompositor_set_user_data(subcompositor: *mut wl_subcompositor, data: *mut c_void) {
    ffi_dispatch!(WCH, wl_proxy_set_user_data,subcompositor as *mut wl_proxy, data)
}

#[inline(always)]
pub unsafe fn wl_subcompositor_get_user_data(subcompositor: *mut wl_subcompositor) -> *mut c_void {
    ffi_dispatch!(WCH, wl_proxy_get_user_data,subcompositor as *mut wl_proxy)
}

#[inline(always)]
pub unsafe fn wl_subcompositor_destroy(subcompositor: *mut wl_subcompositor) {
    ffi_dispatch!(WCH, wl_proxy_marshal,subcompositor as *mut wl_proxy, WL_SUBCOMPOSITOR_DESTROY);
    ffi_dispatch!(WCH, wl_proxy_destroy,subcompositor as *mut wl_proxy)
}

#[inline(always)]
pub unsafe fn wl_subcompositor_get_subsurface(subcompositor: *mut wl_subcompositor,
                                              surface: *mut wl_surface,
                                              parent: *mut wl_surface
                                             ) -> *mut wl_subsurface {
    ffi_dispatch!(WCH, wl_proxy_marshal_constructor,
        subcompositor as *mut wl_proxy,
        WL_SUBCOMPOSITOR_GET_SUBSURFACE,
        ffi_dispatch_static!(WCH,wl_subsurface_interface),
        ptr::null_mut::<c_void>(),
        surface,
        parent
    ) as *mut wl_subsurface
}
