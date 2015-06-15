use std::ptr;

use libc::{c_int, c_void, uint32_t, int32_t};

use ffi::abi::wl_proxy;
#[cfg(not(feature = "dlopen"))]
use ffi::abi::{wl_proxy_destroy, wl_proxy_add_listener, wl_proxy_set_user_data,
               wl_proxy_get_user_data, wl_proxy_marshal_constructor,
               wl_shm_pool_interface};
#[cfg(feature = "dlopen")]
use ffi::abi::WAYLAND_CLIENT_HANDLE as WCH;

use super::shm_pool::wl_shm_pool;

#[repr(C)] pub struct wl_shm;

#[repr(C)]
pub struct wl_shm_listener {
    pub format: extern fn(data: *mut c_void,
                          shm: *mut wl_shm,
                          format: uint32_t
                         )
}

const WL_SHM_CREATE_POOL: uint32_t = 0;

#[inline(always)]
pub unsafe fn wl_shm_add_listener(shm: *mut wl_shm,
                                  listener: *const wl_shm_listener,
                                  data: *mut c_void
                                 ) -> c_int {
    ffi_dispatch!(WCH, wl_proxy_add_listener,
        shm as *mut wl_proxy,
        listener as *mut extern fn(),
        data
    )
}

#[inline(always)]
pub unsafe fn wl_shm_set_user_data(shm: *mut wl_shm, data: *mut c_void) {
    ffi_dispatch!(WCH, wl_proxy_set_user_data,shm as *mut wl_proxy, data)
}

#[inline(always)]
pub unsafe fn wl_shm_get_user_data(shm: *mut wl_shm) -> *mut c_void {
    ffi_dispatch!(WCH, wl_proxy_get_user_data,shm as *mut wl_proxy)
}

#[inline(always)]
pub unsafe fn wl_shm_destroy(shm: *mut wl_shm) {
    ffi_dispatch!(WCH, wl_proxy_destroy,shm as *mut wl_proxy)
}

#[inline(always)]
pub unsafe fn wl_shm_create_pool(shm: *mut wl_shm,
                                 fd: int32_t,
                                 size: int32_t
                                ) -> *mut wl_shm_pool {
    ffi_dispatch!(WCH, wl_proxy_marshal_constructor,
        shm as *mut wl_proxy,
        WL_SHM_CREATE_POOL,
        ffi_dispatch_static!(WCH, wl_shm_pool_interface),
        ptr::null_mut::<c_void>(),
        fd,
        size
    ) as *mut wl_shm_pool
}