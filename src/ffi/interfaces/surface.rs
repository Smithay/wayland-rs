use std::ptr;

use libc::{c_int, c_void, int32_t, uint32_t};

use ffi::abi::{self, wl_proxy};

use super::buffer::wl_buffer;
use super::callback::wl_callback;
use super::output::wl_output;
use super::region::wl_region;

#[repr(C)] pub struct wl_surface;

#[repr(C)]
pub struct wl_surface_listener {
    pub enter: extern fn(data: *mut c_void,
                         surface: *mut wl_surface,
                         output: *mut wl_output
                        ),
    pub leave: extern fn(data: *mut c_void,
                         surface: *mut wl_surface,
                         output: *mut wl_output
                        )
}

const WL_SURFACE_DESTROY: uint32_t = 0;
const WL_SURFACE_ATTACH: uint32_t = 1;
const WL_SURFACE_DAMAGE: uint32_t = 2;
const WL_SURFACE_FRAME: uint32_t = 3;
const WL_SURFACE_SET_OPAQUE_REGION: uint32_t = 4;
const WL_SURFACE_SET_INPUT_REGION: uint32_t = 5;
const WL_SURFACE_COMMIT: uint32_t = 6;
const WL_SURFACE_SET_BUFFER_TRANSFORM: uint32_t = 7;
const WL_SURFACE_SET_BUFFER_SCALE: uint32_t = 8;

#[inline(always)]
pub unsafe fn wl_surface_add_listener(surface: *mut wl_surface,
                                      listener: *const wl_surface_listener,
                                      data: *mut c_void
                                     ) -> c_int {
    abi::wl_proxy_add_listener(
        surface as *mut wl_proxy,
        listener as *mut extern fn(),
        data
    )
}

#[inline(always)]
pub unsafe fn wl_surface_set_user_data(surface: *mut wl_surface, data: *mut c_void) {
    abi::wl_proxy_set_user_data(surface as *mut wl_proxy, data)
}

#[inline(always)]
pub unsafe fn wl_surface_get_user_data(surface: *mut wl_surface) -> *mut c_void {
    abi::wl_proxy_get_user_data(surface as *mut wl_proxy)
}

#[inline(always)]
pub unsafe fn wl_surface_destroy(surface: *mut wl_surface) {
    abi::wl_proxy_marshal(surface as *mut wl_proxy, WL_SURFACE_DESTROY);
    abi::wl_proxy_destroy(surface as *mut wl_proxy)
}

#[inline(always)]
pub unsafe fn wl_surface_attach(surface: *mut wl_surface,
                                buffer: *mut wl_buffer,
                                x: int32_t,
                                y: int32_t
                               ) {
    abi::wl_proxy_marshal(
        surface as *mut wl_proxy,
        WL_SURFACE_ATTACH,
        buffer,
        x,
        y
    )
}

#[inline(always)]
pub unsafe fn wl_surface_damage(surface: *mut wl_surface,
                                x: int32_t,
                                y: int32_t,
                                width: int32_t,
                                height: int32_t
                               ) {
    abi::wl_proxy_marshal(
        surface as *mut wl_proxy,
        WL_SURFACE_DAMAGE,
        x,
        y,
        width,
        height
    )
}

#[inline(always)]
pub unsafe fn wl_surface_frame(surface: *mut wl_surface) -> *mut wl_callback {
    abi::wl_proxy_marshal_constructor(
        surface as *mut wl_proxy,
        WL_SURFACE_FRAME,
        &abi::wl_callback_interface,
        ptr::null_mut::<c_void>()
    ) as *mut wl_callback
}

#[inline(always)]
pub unsafe fn wl_surface_set_opaque_region(surface: *mut wl_surface, region: *mut wl_region) {
    abi::wl_proxy_marshal(surface as *mut wl_proxy, WL_SURFACE_SET_OPAQUE_REGION, region)
}

#[inline(always)]
pub unsafe fn wl_surface_set_input_region(surface: *mut wl_surface, region: *mut wl_region) {
    abi::wl_proxy_marshal(surface as *mut wl_proxy, WL_SURFACE_SET_INPUT_REGION, region)
}

#[inline(always)]
pub unsafe fn wl_surface_commit(surface: *mut wl_surface) {
    abi::wl_proxy_marshal(surface as *mut wl_proxy, WL_SURFACE_COMMIT)
}

#[inline(always)]
pub unsafe fn wl_surface_set_buffer_transform(surface: *mut wl_surface, transform: int32_t) {
    abi::wl_proxy_marshal(surface as *mut wl_proxy, WL_SURFACE_SET_BUFFER_TRANSFORM, transform)
}

#[inline(always)]
pub unsafe fn wl_surface_set_buffer_scale(surface: *mut wl_surface, scale: int32_t) {
    abi::wl_proxy_marshal(surface as *mut wl_proxy, WL_SURFACE_SET_BUFFER_SCALE, scale)
}