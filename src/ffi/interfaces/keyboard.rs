use libc::{c_int, c_void, uint32_t, int32_t};

use ffi::abi::{wl_proxy, wl_array};
use ffi::abi::WAYLAND_CLIENT_HANDLE as WCH;

use super::surface::wl_surface;

#[repr(C)] pub struct wl_keyboard;

#[repr(C)]
pub struct wl_keyboard_listener {
    pub keymap: extern fn(data: *mut c_void,
                          keyboard: *mut wl_keyboard,
                          format: uint32_t,
                          fd: int32_t,
                          size: uint32_t
                         ),
    pub enter: extern fn(data: *mut c_void,
                         keyboard: *mut wl_keyboard,
                         serial: uint32_t,
                         surface: *mut wl_surface,
                         keys: *mut wl_array
                        ),
    pub leave: extern fn(data: *mut c_void,
                         keyboard: *mut wl_keyboard,
                         serial: uint32_t,
                         surface: *mut wl_surface
                        ),
    pub key: extern fn(data: *mut c_void,
                       keyboard: *mut wl_keyboard,
                       serial: uint32_t,
                       time: uint32_t,
                       key: uint32_t,
                       state: uint32_t
                      ),
    pub modifiers: extern fn(data: *mut c_void,
                             keyboard: *mut wl_keyboard,
                             serial: uint32_t,
                             mods_depressed: uint32_t,
                             mods_latched: uint32_t,
                             mods_locked: uint32_t,
                             group: uint32_t
                            ),
    pub repeat_info: extern fn(data: *mut c_void,
                               keyboard: *mut wl_keyboard,
                               rate: int32_t,
                               delay: int32_t
                               )
}

const WL_KEYBOARD_RELEASE: uint32_t = 0;

#[inline(always)]
pub unsafe fn wl_keyboard_add_listener(keyboard: *mut wl_keyboard,
                                       listener: *const wl_keyboard_listener,
                                       data: *mut c_void
                                      ) -> c_int {
    (WCH.wl_proxy_add_listener)(
        keyboard as *mut wl_proxy,
        listener as *mut extern fn(),
        data
    )
}

#[inline(always)]
pub unsafe fn wl_keyboard_set_user_data(keyboard: *mut wl_keyboard, data: *mut c_void) {
    (WCH.wl_proxy_set_user_data)(keyboard as *mut wl_proxy, data)
}

#[inline(always)]
pub unsafe fn wl_keyboard_get_user_data(keyboard: *mut wl_keyboard) -> *mut c_void {
    (WCH.wl_proxy_get_user_data)(keyboard as *mut wl_proxy)
}

#[inline(always)]
pub unsafe fn wl_keyboard_destroy(keyboard: *mut wl_keyboard) {
    (WCH.wl_proxy_destroy)(keyboard as *mut wl_proxy)
}

#[inline(always)]
pub unsafe fn wl_keyboard_release(keyboard: *mut wl_keyboard) {
    (WCH.wl_proxy_marshal)(keyboard as *mut wl_proxy, WL_KEYBOARD_RELEASE);
    (WCH.wl_proxy_destroy)(keyboard as *mut wl_proxy)
}