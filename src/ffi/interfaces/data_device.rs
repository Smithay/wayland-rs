use libc::{c_int, c_void, uint32_t};

use ffi::abi::{wl_proxy, wl_fixed_t};
use ffi::abi::WAYLAND_CLIENT_HANDLE as WCH;

use super::data_offer::wl_data_offer;
use super::data_source::wl_data_source;
use super::surface::wl_surface;

#[repr(C)] pub struct wl_data_device;

#[repr(C)]
pub struct wl_data_device_listener {
    pub data_offer: extern fn(data: *mut c_void,
                              data_device: *mut wl_data_device,
                              id: *mut wl_data_offer
                             ),
    pub enter: extern fn(data: *mut c_void,
                         data_device: *mut wl_data_device,
                         serial: uint32_t,
                         surface: *mut wl_surface,
                         x: wl_fixed_t,
                         y: wl_fixed_t,
                         id: *mut wl_data_offer
                        ),
    pub leave: extern fn(data: *mut c_void,
                         data_device: *mut wl_data_device
                        ),
    pub motion: extern fn(data: *mut c_void,
                          data_device: *mut wl_data_device,
                          time: uint32_t,
                          x: wl_fixed_t,
                          y: wl_fixed_t
                         ),
    pub drop: extern fn(data: *mut c_void,
                        data_device: *mut wl_data_device
                       ),
    pub selection: extern fn(data: *mut c_void,
                             data_device: *mut wl_data_device,
                             id: *mut wl_data_offer
                            )
}

const WL_DATA_DEVICE_START_DRAG: uint32_t = 0;
const WL_DATA_DEVICE_SET_SELECTION: uint32_t = 1;
const WL_DATA_DEVICE_RELEASE: uint32_t = 2;

#[inline(always)]
pub unsafe fn wl_data_device_add_listener(data_device: *mut wl_data_device,
                                          listener: *const wl_data_device_listener,
                                          data: *mut c_void
                                         ) -> c_int {
    (WCH.wl_proxy_add_listener)(
        data_device as *mut wl_proxy,
        listener as *mut extern fn(),
        data
    )
}

#[inline(always)]
pub unsafe fn wl_data_device_set_user_data(data_device: *mut wl_data_device,
                                           data: *mut c_void
                                          ) {
    (WCH.wl_proxy_set_user_data)(data_device as *mut wl_proxy, data)
}

#[inline(always)]
pub unsafe fn wl_data_device_get_user_data(data_device: *mut wl_data_device) -> *mut c_void {
    (WCH.wl_proxy_get_user_data)(data_device as *mut wl_proxy)
}

#[inline(always)]
pub unsafe fn wl_data_device_destroy(data_device: *mut wl_data_device) {
    (WCH.wl_proxy_destroy)(data_device as *mut wl_proxy)
}

#[inline(always)]
pub unsafe fn wl_data_device_start_drag(data_device: *mut wl_data_device,
                                        source: *mut wl_data_source,
                                        origin: *mut wl_surface,
                                        icon: *mut wl_surface,
                                        serial: uint32_t
                                       ) {
    (WCH.wl_proxy_marshal)(
        data_device as *mut wl_proxy,
        WL_DATA_DEVICE_START_DRAG,
        source,
        origin,
        icon,
        serial
    )
}

#[inline(always)]
pub unsafe fn wl_data_device_set_selection(data_device: *mut wl_data_device,
                                           source: *mut wl_data_source,
                                           serial: uint32_t
                                          ) {
    (WCH.wl_proxy_marshal)(
        data_device as *mut wl_proxy,
        WL_DATA_DEVICE_SET_SELECTION,
        source,
        serial
    )
}

#[inline(always)]
pub unsafe fn wl_data_device_release(data_device: *mut wl_data_device) {
    (WCH.wl_proxy_marshal)(data_device as *mut wl_proxy, WL_DATA_DEVICE_RELEASE);
    (WCH.wl_proxy_destroy)(data_device as *mut wl_proxy)
}