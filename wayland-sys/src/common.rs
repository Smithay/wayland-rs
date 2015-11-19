//! Various types and functions that are used by both the client and the server
//! libraries.

use std::os::raw::{c_char, c_void, c_int};

#[repr(C)]
pub struct wl_message {
    pub name: *const c_char,
    pub signature: *const c_char,
    pub types: *const *const wl_interface
}

#[repr(C)]
pub struct wl_interface {
    pub name: *const c_char,
    pub version: c_int,
    pub request_count: c_int,
    pub requests: *const wl_message,
    pub event_count: c_int,
    pub events: *const wl_message
}

#[repr(C)]
pub struct wl_list {
    pub prev: *mut wl_list,
    pub next: *mut wl_list
}

#[repr(C)]
pub struct wl_array {
    pub size: usize,
    pub alloc: usize,
    pub data: *mut c_void
}

pub type wl_fixed_t = i32;

pub fn wl_fixed_to_double(f: wl_fixed_t) -> f64 {
    f as f64 / 256.
}

pub fn wl_fixed_from_double(d: f64) -> wl_fixed_t {
    (d * 256.) as i32
}

pub fn wl_fixed_to_int(f: wl_fixed_t) -> i32 {
    f / 256
}

pub fn wl_fixed_from_int(i: i32) -> wl_fixed_t {
    i * 256
}

// must be the appropriate size
// can contain i32, u32 and pointers
#[repr(C)]
pub struct wl_argument { _f: usize }

pub type wl_dispatcher_func_t = extern "C" fn(*const c_void, *mut c_void, u32, *const wl_message, *const wl_argument);
pub type wl_log_func_t = extern "C" fn(*const c_char, ...);
