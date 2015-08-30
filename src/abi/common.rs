use libc::{c_char, c_void, c_int, size_t};

#[repr(C)]
pub struct wl_message {
    name: *const c_char,
    signature: *const c_char,
    types: *const *const wl_interface
}

#[repr(C)]
pub struct wl_interface {
    name: *const c_char,
    version: c_int,
    method_count: c_int,
    methods: *const wl_message,
    event_count: c_int,
    events: *const wl_message
}

#[repr(C)]
pub struct wl_list {
    prev: *mut wl_list,
    next: *mut wl_list
}

#[repr(C)]
pub struct wl_array {
    size: size_t,
    alloc: size_t,
    data: *mut c_void
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

pub enum wl_argument { }

pub type wl_dispatcher_func_t = extern "C" fn(*const c_void, *mut c_void, u32, *const wl_message, *mut wl_argument);
pub type wl_log_func_t = extern "C" fn(*const c_char, ...);