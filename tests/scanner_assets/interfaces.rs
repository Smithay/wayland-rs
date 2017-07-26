//
// This file was auto-generated, do not edit directly
//

/*
This is an example copyright.
  It contains several lines.
  AS WELL AS ALL CAPS TEXT.
*/

use std::os::raw::{c_char, c_void};
use wayland_sys::common::*;

const NULLPTR: *const c_void = 0 as *const c_void;

static mut types_null: [*const wl_interface; 0] = [
];

// wl_foo
pub static mut wl_foo_interface: wl_interface = wl_interface {
    name: b"wl_foo\0" as *const u8 as *const c_char,
    version: 3,
    request_count: 0,
    requests: NULLPTR as *const wl_message,
    event_count: 0,
    events: NULLPTR as *const wl_message,
};

// wl_bar
pub static mut wl_bar_interface: wl_interface = wl_interface {
    name: b"wl_bar\0" as *const u8 as *const c_char,
    version: 1,
    request_count: 0,
    requests: NULLPTR as *const wl_message,
    event_count: 0,
    events: NULLPTR as *const wl_message,
};
