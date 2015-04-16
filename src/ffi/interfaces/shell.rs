use std::ptr;

use libc::{c_void, uint32_t};

use ffi::abi::{self, wl_proxy};

use super::shell_surface::wl_shell_surface;
use super::surface::wl_surface;

#[repr(C)] pub struct wl_shell;

const WL_SHELL_GET_SHELL_SURFACE: uint32_t = 0;

#[inline(always)]
pub unsafe fn wl_shell_set_user_data(shell: *mut wl_shell, data: *mut c_void) {
    abi::wl_proxy_set_user_data(shell as *mut wl_proxy, data)
}

#[inline(always)]
pub unsafe fn wl_shell_get_user_data(shell: *mut wl_shell) -> *mut c_void {
    abi::wl_proxy_get_user_data(shell as *mut wl_proxy)
}

#[inline(always)]
pub unsafe fn wl_shell_destroy(shell: *mut wl_shell) {
    abi::wl_proxy_destroy(shell as *mut wl_proxy)
}

#[inline(always)]
pub unsafe fn wl_shell_get_shell_surface(shell: *mut wl_shell,
                                         surface: *mut wl_surface
                                        ) -> *mut wl_shell_surface {
    abi::wl_proxy_marshal_constructor(
        shell as *mut wl_proxy,
        WL_SHELL_GET_SHELL_SURFACE,
        &abi::wl_shell_surface_interface,
        ptr::null_mut::<c_void>(),
        surface
    ) as *mut wl_shell_surface
}