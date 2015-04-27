use libc::{c_int, c_void, c_char, uint32_t, int32_t};

use ffi::abi::wl_proxy;
use ffi::abi::WAYLAND_CLIENT_HANDLE as WCH;

#[repr(C)] pub struct wl_data_offer;

#[repr(C)]
pub struct wl_data_offer_listener {
    pub offer: extern fn(data: *mut c_void,
                         wl_data_offer: *mut wl_data_offer,
                         mime_type: *const c_char
                        )
}

const WL_DATA_OFFER_ACCEPT: uint32_t = 0;
const WL_DATA_OFFER_RECEIVE: uint32_t = 1;
const WL_DATA_OFFER_DESTROY: uint32_t = 2;

#[inline(always)]
pub unsafe fn wl_data_offer_add_listener(data_offer: *mut wl_data_offer,
                                         listener: *const wl_data_offer_listener,
                                         data: *mut c_void
                                        ) -> c_int {
    (WCH.wl_proxy_add_listener)(
        data_offer as *mut wl_proxy,
        listener as *mut extern fn(),
        data
    )
}

#[inline(always)]
pub unsafe fn wl_data_offer_set_user_data(data_offer: *mut wl_data_offer, data: *mut c_void) {
    (WCH.wl_proxy_set_user_data)(data_offer as *mut wl_proxy, data)
}

#[inline(always)]
pub unsafe fn wl_data_offer_get_user_data(data_offer: *mut wl_data_offer) -> *mut c_void {
    (WCH.wl_proxy_get_user_data)(data_offer as *mut wl_proxy)
}

#[inline(always)]
pub unsafe fn wl_data_offer_accept(data_offer: *mut wl_data_offer,
                                   serial: uint32_t,
                                   mime_type: *const c_char
                                  ) {
    (WCH.wl_proxy_marshal)(
        data_offer as *mut wl_proxy,
        WL_DATA_OFFER_ACCEPT,
        serial,
        mime_type
    )
}

#[inline(always)]
pub unsafe fn wl_data_offer_receive(data_offer: *mut wl_data_offer,
                                    mime_type: *const c_char,
                                    fd: int32_t
                                   ) {
    (WCH.wl_proxy_marshal)(
        data_offer as *mut wl_proxy,
        WL_DATA_OFFER_RECEIVE,
        mime_type,
        fd
    )
}

#[inline(always)]
pub unsafe fn wl_data_offer_destroy(data_offer: *mut wl_data_offer) {
    (WCH.wl_proxy_marshal)(data_offer as *mut wl_proxy, WL_DATA_OFFER_DESTROY);
    (WCH.wl_proxy_destroy)(data_offer as *mut wl_proxy)
}