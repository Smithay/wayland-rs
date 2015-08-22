use libc::{c_int, c_void, c_char, uint32_t, int32_t};

use ffi::abi::wl_proxy;
#[cfg(not(feature = "dlopen"))]
use ffi::abi::{wl_proxy_destroy, wl_proxy_add_listener, wl_proxy_set_user_data,
               wl_proxy_get_user_data, wl_proxy_marshal};
#[cfg(feature = "dlopen")]
use ffi::abi::WAYLAND_CLIENT_HANDLE as WCH;
use ffi::enums::FullscreenMethod;
use ffi::enums::ShellSurfaceResize;

use super::output::wl_output;
use super::seat::wl_seat;
use super::surface::wl_surface;

pub enum wl_shell_surface { }

#[repr(C)]
pub struct wl_shell_surface_listener {
    pub ping: extern fn(data: *mut c_void,
                        shell_surface: *mut wl_shell_surface,
                        serial: uint32_t
                       ),
    pub configure: extern fn(data: *mut c_void,
                             shell_surface: *mut wl_shell_surface,
                             edges: ShellSurfaceResize,
                             width: int32_t,
                             height: int32_t
                            ),
    pub popup_done: extern fn(data: *mut c_void,
                              shell_surface: *mut wl_shell_surface,
                             )   
}

const WL_SHELL_SURFACE_PONG: uint32_t = 0;
const WL_SHELL_SURFACE_MOVE: uint32_t = 1;
const WL_SHELL_SURFACE_RESIZE: uint32_t = 2;
const WL_SHELL_SURFACE_SET_TOPLEVEL: uint32_t = 3;
const WL_SHELL_SURFACE_SET_TRANSIENT: uint32_t = 4;
const WL_SHELL_SURFACE_SET_FULLSCREEN: uint32_t = 5;
const WL_SHELL_SURFACE_SET_POPUP: uint32_t = 6;
const WL_SHELL_SURFACE_SET_MAXIMIZED: uint32_t = 7;
const WL_SHELL_SURFACE_SET_TITLE: uint32_t = 8;
const WL_SHELL_SURFACE_SET_CLASS: uint32_t = 9;

#[inline(always)]
pub unsafe fn wl_shell_surface_add_listener(shell_surface: *mut wl_shell_surface,
                                            listener: *const wl_shell_surface_listener,
                                            data: *mut c_void
                                           ) -> c_int {
    ffi_dispatch!(WCH, wl_proxy_add_listener,
        shell_surface as *mut wl_proxy,
        listener as *mut extern fn(),
        data
    )
}

#[inline(always)]
pub unsafe fn wl_shell_surface_set_user_data(shell_surface: *mut wl_shell_surface,
                                             data: *mut c_void
                                            ) {
    ffi_dispatch!(WCH, wl_proxy_set_user_data,shell_surface as *mut wl_proxy, data)
}

#[inline(always)]
pub unsafe fn wl_shell_surface_get_user_data(shell_surface: *mut wl_shell_surface) -> *mut c_void {
    ffi_dispatch!(WCH, wl_proxy_get_user_data,shell_surface as *mut wl_proxy)
}

#[inline(always)]
pub unsafe fn wl_shell_surface_destroy(shell_surface: *mut wl_shell_surface) {
    ffi_dispatch!(WCH, wl_proxy_destroy,shell_surface as *mut wl_proxy)
}

#[inline(always)]
pub unsafe fn wl_shell_surface_pong(shell_surface: *mut wl_shell_surface, serial: uint32_t) {
    ffi_dispatch!(WCH, wl_proxy_marshal,shell_surface as *mut wl_proxy, WL_SHELL_SURFACE_PONG, serial)
}

#[inline(always)]
pub unsafe fn wl_shell_surface_move(shell_surface: *mut wl_shell_surface,
                                    seat: *mut wl_seat,
                                    serial: uint32_t
                                   ) {
    ffi_dispatch!(WCH, wl_proxy_marshal,
        shell_surface as *mut wl_proxy,
        WL_SHELL_SURFACE_MOVE,
        seat,
        serial
    )
}

#[inline(always)]
pub unsafe fn wl_shell_surface_resize(shell_surface: *mut wl_shell_surface,
                                      seat: *mut wl_seat,
                                      serial: uint32_t,
                                      edges: uint32_t
                                     ) {
    ffi_dispatch!(WCH, wl_proxy_marshal,
        shell_surface as *mut wl_proxy,
        WL_SHELL_SURFACE_RESIZE,
        seat,
        serial,
        edges
    )
}

#[inline(always)]
pub unsafe fn wl_shell_surface_set_toplevel(shell_surface: *mut wl_shell_surface) {
    ffi_dispatch!(WCH, wl_proxy_marshal,shell_surface as *mut wl_proxy, WL_SHELL_SURFACE_SET_TOPLEVEL)
}

#[inline(always)]
pub unsafe fn wl_shell_surface_set_transient(shell_surface: *mut wl_shell_surface,
                                             parent: *mut wl_surface,
                                             x: int32_t,
                                             y: int32_t,
                                             flags: uint32_t
                                            ) {
    ffi_dispatch!(WCH, wl_proxy_marshal,
        shell_surface as *mut wl_proxy,
        WL_SHELL_SURFACE_SET_TRANSIENT,
        parent,
        x,
        y,
        flags
    )
}

#[inline(always)]
pub unsafe fn wl_shell_surface_set_fullscreen(shell_surface: *mut wl_shell_surface,
                                              method: FullscreenMethod,
                                              framerate: uint32_t,
                                              output: *mut wl_output
                                             ) {
    ffi_dispatch!(WCH, wl_proxy_marshal,
        shell_surface as *mut wl_proxy,
        WL_SHELL_SURFACE_SET_FULLSCREEN,
        method,
        framerate,
        output
    )
}

#[inline(always)]
pub unsafe fn wl_shell_surface_set_popup(shell_surface: *mut wl_shell_surface,
                                         seat: *mut wl_seat,
                                         serial: uint32_t,
                                         parent: *mut wl_surface,
                                         x: int32_t,
                                         y: int32_t,
                                         flags: uint32_t
                                        ) {
    ffi_dispatch!(WCH, wl_proxy_marshal,
        shell_surface as *mut wl_proxy,
        WL_SHELL_SURFACE_SET_POPUP,
        seat,
        serial,
        parent,
        x,
        y,
        flags
    )
}

#[inline(always)]
pub unsafe fn wl_shell_surface_set_maximized(shell_surface: *mut wl_shell_surface,
                                             output: *mut wl_output
                                            ) {
    ffi_dispatch!(WCH, wl_proxy_marshal,
        shell_surface as *mut wl_proxy,
        WL_SHELL_SURFACE_SET_MAXIMIZED,
        output
    )
}

#[inline(always)]
pub unsafe fn wl_shell_surface_set_title(shell_surface: *mut wl_shell_surface,
                                         title: *const c_char
                                        ) {
    ffi_dispatch!(WCH, wl_proxy_marshal,
        shell_surface as *mut wl_proxy,
        WL_SHELL_SURFACE_SET_TITLE,
        title
    )
}

#[inline(always)]
pub unsafe fn wl_shell_surface_set_class(shell_surface: *mut wl_shell_surface,
                                         class_: *const c_char
                                        ) {
    ffi_dispatch!(WCH, wl_proxy_marshal,
        shell_surface as *mut wl_proxy,
        WL_SHELL_SURFACE_SET_CLASS,
        class_
    )
}