use std::ptr;

use libc::{c_void, uint32_t};

use ffi::abi::wl_proxy;
use ffi::abi::WAYLAND_CLIENT_HANDLE as WCH;

use super::data_device::wl_data_device;
use super::data_source::wl_data_source;
use super::seat::wl_seat;

#[repr(C)] pub struct wl_data_device_manager;

const WL_DATA_DEVICE_MANAGER_CREATE_DATA_SOURCE: uint32_t = 0;
const WL_DATA_DEVICE_MANAGER_GET_DATA_DEVICE: uint32_t = 1;

#[inline(always)]
pub unsafe fn wl_data_device_manager_set_user_data(ddm: *mut wl_data_device_manager,
                                                   data: *mut c_void
                                                  ) {
    (WCH.wl_proxy_set_user_data)(ddm as *mut wl_proxy, data)
}

#[inline(always)]
pub unsafe fn wl_data_device_manager_get_user_data(ddm: *mut wl_data_device_manager
                                                  ) -> *mut c_void {
    (WCH.wl_proxy_get_user_data)(ddm as *mut wl_proxy)
}

#[inline(always)]
pub unsafe fn wl_data_device_manager_destroy(ddm: *mut wl_data_device_manager) {
    (WCH.wl_proxy_destroy)(ddm as *mut wl_proxy)
}

#[inline(always)]
pub unsafe fn wl_data_device_manager_create_data_source(ddm: *mut wl_data_device_manager
                                                       ) -> *mut wl_data_source {
    (WCH.wl_proxy_marshal_constructor)(
        ddm as *mut wl_proxy,
        WL_DATA_DEVICE_MANAGER_CREATE_DATA_SOURCE,
        WCH.wl_data_source_interface,
        ptr::null_mut::<c_void>()
    ) as *mut wl_data_source
}

#[inline(always)]
pub unsafe fn wl_data_device_manager_get_data_device(ddm: *mut wl_data_device_manager,
                                                     seat: *mut wl_seat
                                                    ) -> *mut wl_data_device {
    (WCH.wl_proxy_marshal_constructor)(
        ddm as *mut wl_proxy,
        WL_DATA_DEVICE_MANAGER_GET_DATA_DEVICE,
        WCH.wl_data_device_interface,
        ptr::null_mut::<c_void>(),
        seat
    ) as *mut wl_data_device
}