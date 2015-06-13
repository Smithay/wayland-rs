use std::ptr;

use libc::{c_void, int32_t, uint32_t};

use ffi::abi::wl_proxy;
#[cfg(not(feature = "dlopen"))]
use ffi::abi::{wl_proxy_destroy, wl_proxy_set_user_data, wl_proxy_get_user_data,
               wl_proxy_marshal, wl_proxy_marshal_constructor, wl_buffer_interface};
#[cfg(feature = "dlopen")]
use ffi::abi::WAYLAND_CLIENT_HANDLE as WCH;

use super::buffer::wl_buffer;

#[repr(C)] pub struct wl_shm_pool;

const WL_SHM_POOL_CREATE_BUFFER: uint32_t = 0;
const WL_SHM_POOL_DESTROY: uint32_t = 1;
const WL_SHM_POOL_RESIZE: uint32_t = 2;

#[inline(always)]
pub unsafe fn wl_shm_pool_set_user_data(shm_pool: *mut wl_shm_pool, data: *mut c_void) {
    ffi_dispatch!(WCH, wl_proxy_set_user_data,shm_pool as *mut wl_proxy, data)
}

#[inline(always)]
pub unsafe fn wl_shm_pool_get_user_data(shm_pool: *mut wl_shm_pool) -> *mut c_void {
    ffi_dispatch!(WCH, wl_proxy_get_user_data,shm_pool as *mut wl_proxy)
}

#[inline(always)]
pub unsafe fn wl_shm_pool_create_buffer(shm_pool: *mut wl_shm_pool,
                                        offset: int32_t,
                                        width: int32_t,
                                        height: int32_t,
                                        stride: int32_t,
                                        format: uint32_t
                                       ) -> *mut wl_buffer {
    ffi_dispatch!(WCH, wl_proxy_marshal_constructor,
        shm_pool as *mut wl_proxy,
        WL_SHM_POOL_CREATE_BUFFER,
        ffi_dispatch_static!(WCH, wl_buffer_interface),
        ptr::null_mut::<c_void>(),
        offset,
        width,
        height,
        stride,
        format
    ) as *mut wl_buffer
}

#[inline(always)]
pub unsafe fn wl_shm_pool_destroy(shm_pool: *mut wl_shm_pool) {
    ffi_dispatch!(WCH, wl_proxy_marshal,shm_pool as *mut wl_proxy, WL_SHM_POOL_DESTROY);
    ffi_dispatch!(WCH, wl_proxy_destroy,shm_pool as *mut wl_proxy)
}

#[inline(always)]
pub unsafe fn wl_shm_pool_resize(shm_pool: *mut wl_shm_pool, size: int32_t) {
    ffi_dispatch!(WCH, wl_proxy_marshal,shm_pool as *mut wl_proxy, WL_SHM_POOL_RESIZE, size)
}