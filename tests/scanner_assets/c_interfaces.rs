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

static mut types_null: [*const wl_interface; 5] = [
    NULLPTR as *const wl_interface,
    NULLPTR as *const wl_interface,
    NULLPTR as *const wl_interface,
    NULLPTR as *const wl_interface,
    NULLPTR as *const wl_interface,
];

// wl_foo

static mut wl_foo_requests_create_bar_types: [*const wl_interface; 1] = [
    unsafe { &wl_bar_interface as *const wl_interface },
];
pub static mut wl_foo_requests: [wl_message; 2] = [
    wl_message { name: b"foo_it\0" as *const u8 as *const c_char, signature: b"iusfh\0" as *const u8 as *const c_char, types: unsafe { &types_null as *const _ } },
    wl_message { name: b"create_bar\0" as *const u8 as *const c_char, signature: b"n\0" as *const u8 as *const c_char, types: unsafe { &wl_foo_requests_create_bar_types as *const _ } },
];
pub static mut wl_foo_events: [wl_message; 1] = [
    wl_message { name: b"cake\0" as *const u8 as *const c_char, signature: b"2uu\0" as *const u8 as *const c_char, types: unsafe { &types_null as *const _ } },
];

pub static mut wl_foo_interface: wl_interface = wl_interface {
    name: b"wl_foo\0" as *const u8 as *const c_char,
    version: 3,
    request_count: 2,
    requests: unsafe { &wl_foo_requests as *const _ },
    event_count: 1,
    events: unsafe { &wl_foo_events as *const _ },
};

// wl_bar

static mut wl_bar_requests_bar_delivery_types: [*const wl_interface; 3] = [
    NULLPTR as *const wl_interface,
    unsafe { &wl_foo_interface as *const wl_interface },
    NULLPTR as *const wl_interface,
];
pub static mut wl_bar_requests: [wl_message; 2] = [
    wl_message { name: b"bar_delivery\0" as *const u8 as *const c_char, signature: b"2uoa\0" as *const u8 as *const c_char, types: unsafe { &wl_bar_requests_bar_delivery_types as *const _ } },
    wl_message { name: b"release\0" as *const u8 as *const c_char, signature: b"\0" as *const u8 as *const c_char, types: unsafe { &types_null as *const _ } },
];

pub static mut wl_bar_interface: wl_interface = wl_interface {
    name: b"wl_bar\0" as *const u8 as *const c_char,
    version: 1,
    request_count: 2,
    requests: unsafe { &wl_bar_requests as *const _ },
    event_count: 0,
    events: NULLPTR as *const wl_message,
};

// wl_display


pub static mut wl_display_interface: wl_interface = wl_interface {
    name: b"wl_display\0" as *const u8 as *const c_char,
    version: 1,
    request_count: 0,
    requests: NULLPTR as *const wl_message,
    event_count: 0,
    events: NULLPTR as *const wl_message,
};

// wl_registry

pub static mut wl_registry_requests: [wl_message; 1] = [
    wl_message { name: b"bind\0" as *const u8 as *const c_char, signature: b"usun\0" as *const u8 as *const c_char, types: unsafe { &types_null as *const _ } },
];

pub static mut wl_registry_interface: wl_interface = wl_interface {
    name: b"wl_registry\0" as *const u8 as *const c_char,
    version: 1,
    request_count: 1,
    requests: unsafe { &wl_registry_requests as *const _ },
    event_count: 0,
    events: NULLPTR as *const wl_message,
};

// wl_callback

pub static mut wl_callback_events: [wl_message; 1] = [
    wl_message { name: b"done\0" as *const u8 as *const c_char, signature: b"u\0" as *const u8 as *const c_char, types: unsafe { &types_null as *const _ } },
];

pub static mut wl_callback_interface: wl_interface = wl_interface {
    name: b"wl_callback\0" as *const u8 as *const c_char,
    version: 1,
    request_count: 0,
    requests: NULLPTR as *const wl_message,
    event_count: 1,
    events: unsafe { &wl_callback_events as *const _ },
};

