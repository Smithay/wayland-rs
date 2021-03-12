const NULLPTR: *const std::os::raw::c_void = 0 as *const std::os::raw::c_void;
static mut types_null: [*const wayland_commons::sys::common::wl_interface; 6] = [
    NULLPTR as *const wayland_commons::sys::common::wl_interface,
    NULLPTR as *const wayland_commons::sys::common::wl_interface,
    NULLPTR as *const wayland_commons::sys::common::wl_interface,
    NULLPTR as *const wayland_commons::sys::common::wl_interface,
    NULLPTR as *const wayland_commons::sys::common::wl_interface,
    NULLPTR as *const wayland_commons::sys::common::wl_interface,
];
pub static WL_DISPLAY_INTERFACE: wayland_commons::Interface = wayland_commons::Interface {
    name: "wl_display",
    version: 1u32,
    requests: &[
        wayland_commons::MessageDesc {
            name: "sync",
            signature: &[wayland_commons::ArgumentType::NewId(wayland_commons::AllowNull::No)],
            since: 1u32,
            is_destructor: false,
            child_interface: Some(&WL_CALLBACK_INTERFACE),
            arg_interfaces: &[],
        },
        wayland_commons::MessageDesc {
            name: "get_registry",
            signature: &[wayland_commons::ArgumentType::NewId(wayland_commons::AllowNull::No)],
            since: 1u32,
            is_destructor: false,
            child_interface: Some(&WL_REGISTRY_INTERFACE),
            arg_interfaces: &[],
        },
    ],
    events: &[
        wayland_commons::MessageDesc {
            name: "error",
            signature: &[
                wayland_commons::ArgumentType::Object(wayland_commons::AllowNull::No),
                wayland_commons::ArgumentType::Uint,
                wayland_commons::ArgumentType::Str(wayland_commons::AllowNull::No),
            ],
            since: 1u32,
            is_destructor: false,
            child_interface: None,
            arg_interfaces: &[&wayland_commons::ANONYMOUS_INTERFACE],
        },
        wayland_commons::MessageDesc {
            name: "delete_id",
            signature: &[wayland_commons::ArgumentType::Uint],
            since: 1u32,
            is_destructor: false,
            child_interface: None,
            arg_interfaces: &[],
        },
    ],
    c_ptr: Some(unsafe { &wl_display_interface }),
};
static mut wl_display_requests_sync_types: [*const wayland_commons::sys::common::wl_interface; 1] =
    [unsafe { &wl_callback_interface as *const wayland_commons::sys::common::wl_interface }];
static mut wl_display_requests_get_registry_types:
    [*const wayland_commons::sys::common::wl_interface; 1] =
    [unsafe { &wl_registry_interface as *const wayland_commons::sys::common::wl_interface }];
pub static mut wl_display_requests: [wayland_commons::sys::common::wl_message; 2] = [
    wayland_commons::sys::common::wl_message {
        name: b"sync\0" as *const u8 as *const std::os::raw::c_char,
        signature: b"n\0" as *const u8 as *const std::os::raw::c_char,
        types: unsafe { &wl_display_requests_sync_types as *const _ },
    },
    wayland_commons::sys::common::wl_message {
        name: b"get_registry\0" as *const u8 as *const std::os::raw::c_char,
        signature: b"n\0" as *const u8 as *const std::os::raw::c_char,
        types: unsafe { &wl_display_requests_get_registry_types as *const _ },
    },
];
pub static mut wl_display_events: [wayland_commons::sys::common::wl_message; 2] = [
    wayland_commons::sys::common::wl_message {
        name: b"error\0" as *const u8 as *const std::os::raw::c_char,
        signature: b"ous\0" as *const u8 as *const std::os::raw::c_char,
        types: unsafe { &types_null as *const _ },
    },
    wayland_commons::sys::common::wl_message {
        name: b"delete_id\0" as *const u8 as *const std::os::raw::c_char,
        signature: b"u\0" as *const u8 as *const std::os::raw::c_char,
        types: unsafe { &types_null as *const _ },
    },
];
pub static mut wl_display_interface: wayland_commons::sys::common::wl_interface =
    wayland_commons::sys::common::wl_interface {
        name: b"wl_display\0" as *const u8 as *const std::os::raw::c_char,
        version: 1,
        request_count: 2,
        requests: unsafe { &wl_display_requests as *const _ },
        event_count: 2,
        events: unsafe { &wl_display_events as *const _ },
    };
pub static WL_REGISTRY_INTERFACE: wayland_commons::Interface = wayland_commons::Interface {
    name: "wl_registry",
    version: 1u32,
    requests: &[wayland_commons::MessageDesc {
        name: "bind",
        signature: &[
            wayland_commons::ArgumentType::Uint,
            wayland_commons::ArgumentType::Str(wayland_commons::AllowNull::No),
            wayland_commons::ArgumentType::Uint,
            wayland_commons::ArgumentType::NewId(wayland_commons::AllowNull::No),
        ],
        since: 1u32,
        is_destructor: false,
        child_interface: None,
        arg_interfaces: &[],
    }],
    events: &[
        wayland_commons::MessageDesc {
            name: "global",
            signature: &[
                wayland_commons::ArgumentType::Uint,
                wayland_commons::ArgumentType::Str(wayland_commons::AllowNull::No),
                wayland_commons::ArgumentType::Uint,
            ],
            since: 1u32,
            is_destructor: false,
            child_interface: None,
            arg_interfaces: &[],
        },
        wayland_commons::MessageDesc {
            name: "global_remove",
            signature: &[wayland_commons::ArgumentType::Uint],
            since: 1u32,
            is_destructor: false,
            child_interface: None,
            arg_interfaces: &[],
        },
    ],
    c_ptr: Some(unsafe { &wl_registry_interface }),
};
pub static mut wl_registry_requests: [wayland_commons::sys::common::wl_message; 1] =
    [wayland_commons::sys::common::wl_message {
        name: b"bind\0" as *const u8 as *const std::os::raw::c_char,
        signature: b"usun\0" as *const u8 as *const std::os::raw::c_char,
        types: unsafe { &types_null as *const _ },
    }];
pub static mut wl_registry_events: [wayland_commons::sys::common::wl_message; 2] = [
    wayland_commons::sys::common::wl_message {
        name: b"global\0" as *const u8 as *const std::os::raw::c_char,
        signature: b"usu\0" as *const u8 as *const std::os::raw::c_char,
        types: unsafe { &types_null as *const _ },
    },
    wayland_commons::sys::common::wl_message {
        name: b"global_remove\0" as *const u8 as *const std::os::raw::c_char,
        signature: b"u\0" as *const u8 as *const std::os::raw::c_char,
        types: unsafe { &types_null as *const _ },
    },
];
pub static mut wl_registry_interface: wayland_commons::sys::common::wl_interface =
    wayland_commons::sys::common::wl_interface {
        name: b"wl_registry\0" as *const u8 as *const std::os::raw::c_char,
        version: 1,
        request_count: 1,
        requests: unsafe { &wl_registry_requests as *const _ },
        event_count: 2,
        events: unsafe { &wl_registry_events as *const _ },
    };
pub static WL_CALLBACK_INTERFACE: wayland_commons::Interface = wayland_commons::Interface {
    name: "wl_callback",
    version: 1u32,
    requests: &[],
    events: &[wayland_commons::MessageDesc {
        name: "done",
        signature: &[wayland_commons::ArgumentType::Uint],
        since: 1u32,
        is_destructor: true,
        child_interface: None,
        arg_interfaces: &[],
    }],
    c_ptr: Some(unsafe { &wl_callback_interface }),
};
pub static mut wl_callback_events: [wayland_commons::sys::common::wl_message; 1] =
    [wayland_commons::sys::common::wl_message {
        name: b"done\0" as *const u8 as *const std::os::raw::c_char,
        signature: b"u\0" as *const u8 as *const std::os::raw::c_char,
        types: unsafe { &types_null as *const _ },
    }];
pub static mut wl_callback_interface: wayland_commons::sys::common::wl_interface =
    wayland_commons::sys::common::wl_interface {
        name: b"wl_callback\0" as *const u8 as *const std::os::raw::c_char,
        version: 1,
        request_count: 0,
        requests: NULLPTR as *const wayland_commons::sys::common::wl_message,
        event_count: 1,
        events: unsafe { &wl_callback_events as *const _ },
    };
pub static TEST_GLOBAL_INTERFACE: wayland_commons::Interface = wayland_commons::Interface {
    name: "test_global",
    version: 3u32,
    requests: &[
        wayland_commons::MessageDesc {
            name: "many_args",
            signature: &[
                wayland_commons::ArgumentType::Uint,
                wayland_commons::ArgumentType::Int,
                wayland_commons::ArgumentType::Fixed,
                wayland_commons::ArgumentType::Array(wayland_commons::AllowNull::No),
                wayland_commons::ArgumentType::Str(wayland_commons::AllowNull::No),
                wayland_commons::ArgumentType::Fd,
            ],
            since: 1u32,
            is_destructor: false,
            child_interface: None,
            arg_interfaces: &[],
        },
        wayland_commons::MessageDesc {
            name: "get_secondary",
            signature: &[wayland_commons::ArgumentType::NewId(wayland_commons::AllowNull::No)],
            since: 2u32,
            is_destructor: false,
            child_interface: Some(&SECONDARY_INTERFACE),
            arg_interfaces: &[],
        },
        wayland_commons::MessageDesc {
            name: "get_tertiary",
            signature: &[wayland_commons::ArgumentType::NewId(wayland_commons::AllowNull::No)],
            since: 3u32,
            is_destructor: false,
            child_interface: Some(&TERTIARY_INTERFACE),
            arg_interfaces: &[],
        },
        wayland_commons::MessageDesc {
            name: "link",
            signature: &[
                wayland_commons::ArgumentType::Object(wayland_commons::AllowNull::No),
                wayland_commons::ArgumentType::Object(wayland_commons::AllowNull::Yes),
                wayland_commons::ArgumentType::Uint,
            ],
            since: 3u32,
            is_destructor: false,
            child_interface: None,
            arg_interfaces: &[&SECONDARY_INTERFACE, &TERTIARY_INTERFACE],
        },
    ],
    events: &[
    wayland_commons::MessageDesc {
        name: "many_args_evt",
        signature: &[
            wayland_commons::ArgumentType::Uint,
            wayland_commons::ArgumentType::Int,
            wayland_commons::ArgumentType::Fixed,
            wayland_commons::ArgumentType::Array(wayland_commons::AllowNull::No),
            wayland_commons::ArgumentType::Str(wayland_commons::AllowNull::No),
            wayland_commons::ArgumentType::Fd,
        ],
        since: 1u32,
        is_destructor: false,
        child_interface: None,
        arg_interfaces: &[],
    },
    wayland_commons::MessageDesc {
        name: "ack_secondary",
        signature: &[wayland_commons::ArgumentType::Object(wayland_commons::AllowNull::No)],
        since: 1u32,
        is_destructor: false,
        child_interface: None,
        arg_interfaces: &[&SECONDARY_INTERFACE],
    },
],
    c_ptr: Some(unsafe { &test_global_interface }),
};
static mut test_global_requests_get_secondary_types:
    [*const wayland_commons::sys::common::wl_interface; 1] =
    [unsafe { &secondary_interface as *const wayland_commons::sys::common::wl_interface }];
static mut test_global_requests_get_tertiary_types:
    [*const wayland_commons::sys::common::wl_interface; 1] =
    [unsafe { &tertiary_interface as *const wayland_commons::sys::common::wl_interface }];
static mut test_global_requests_link_types: [*const wayland_commons::sys::common::wl_interface; 3] = [
    unsafe { &secondary_interface as *const wayland_commons::sys::common::wl_interface },
    unsafe { &tertiary_interface as *const wayland_commons::sys::common::wl_interface },
    NULLPTR as *const wayland_commons::sys::common::wl_interface,
];
pub static mut test_global_requests: [wayland_commons::sys::common::wl_message; 4] = [
    wayland_commons::sys::common::wl_message {
        name: b"many_args\0" as *const u8 as *const std::os::raw::c_char,
        signature: b"uifash\0" as *const u8 as *const std::os::raw::c_char,
        types: unsafe { &types_null as *const _ },
    },
    wayland_commons::sys::common::wl_message {
        name: b"get_secondary\0" as *const u8 as *const std::os::raw::c_char,
        signature: b"2n\0" as *const u8 as *const std::os::raw::c_char,
        types: unsafe { &test_global_requests_get_secondary_types as *const _ },
    },
    wayland_commons::sys::common::wl_message {
        name: b"get_tertiary\0" as *const u8 as *const std::os::raw::c_char,
        signature: b"3n\0" as *const u8 as *const std::os::raw::c_char,
        types: unsafe { &test_global_requests_get_tertiary_types as *const _ },
    },
    wayland_commons::sys::common::wl_message {
        name: b"link\0" as *const u8 as *const std::os::raw::c_char,
        signature: b"3o?ou\0" as *const u8 as *const std::os::raw::c_char,
        types: unsafe { &test_global_requests_link_types as *const _ },
    },
];
static mut test_global_events_ack_secondary_types:
    [*const wayland_commons::sys::common::wl_interface; 1] =
    [unsafe { &secondary_interface as *const wayland_commons::sys::common::wl_interface }];
pub static mut test_global_events: [wayland_commons::sys::common::wl_message; 2] = [
    wayland_commons::sys::common::wl_message {
        name: b"many_args_evt\0" as *const u8 as *const std::os::raw::c_char,
        signature: b"uifash\0" as *const u8 as *const std::os::raw::c_char,
        types: unsafe { &types_null as *const _ },
    },
    wayland_commons::sys::common::wl_message {
        name: b"ack_secondary\0" as *const u8 as *const std::os::raw::c_char,
        signature: b"o\0" as *const u8 as *const std::os::raw::c_char,
        types: unsafe { &test_global_events_ack_secondary_types as *const _ },
    },
];
pub static mut test_global_interface: wayland_commons::sys::common::wl_interface =
    wayland_commons::sys::common::wl_interface {
        name: b"test_global\0" as *const u8 as *const std::os::raw::c_char,
        version: 3,
        request_count: 4,
        requests: unsafe { &test_global_requests as *const _ },
        event_count: 2,
        events: unsafe { &test_global_events as *const _ },
    };
pub static SECONDARY_INTERFACE: wayland_commons::Interface = wayland_commons::Interface {
    name: "secondary",
    version: 3u32,
    requests: &[wayland_commons::MessageDesc {
        name: "destroy",
        signature: &[],
        since: 2u32,
        is_destructor: true,
        child_interface: None,
        arg_interfaces: &[],
    }],
    events: &[],
    c_ptr: Some(unsafe { &secondary_interface }),
};
pub static mut secondary_requests: [wayland_commons::sys::common::wl_message; 1] =
    [wayland_commons::sys::common::wl_message {
        name: b"destroy\0" as *const u8 as *const std::os::raw::c_char,
        signature: b"2\0" as *const u8 as *const std::os::raw::c_char,
        types: unsafe { &types_null as *const _ },
    }];
pub static mut secondary_interface: wayland_commons::sys::common::wl_interface =
    wayland_commons::sys::common::wl_interface {
        name: b"secondary\0" as *const u8 as *const std::os::raw::c_char,
        version: 3,
        request_count: 1,
        requests: unsafe { &secondary_requests as *const _ },
        event_count: 0,
        events: NULLPTR as *const wayland_commons::sys::common::wl_message,
    };
pub static TERTIARY_INTERFACE: wayland_commons::Interface = wayland_commons::Interface {
    name: "tertiary",
    version: 3u32,
    requests: &[wayland_commons::MessageDesc {
        name: "destroy",
        signature: &[],
        since: 3u32,
        is_destructor: true,
        child_interface: None,
        arg_interfaces: &[],
    }],
    events: &[],
    c_ptr: Some(unsafe { &tertiary_interface }),
};
pub static mut tertiary_requests: [wayland_commons::sys::common::wl_message; 1] =
    [wayland_commons::sys::common::wl_message {
        name: b"destroy\0" as *const u8 as *const std::os::raw::c_char,
        signature: b"3\0" as *const u8 as *const std::os::raw::c_char,
        types: unsafe { &types_null as *const _ },
    }];
pub static mut tertiary_interface: wayland_commons::sys::common::wl_interface =
    wayland_commons::sys::common::wl_interface {
        name: b"tertiary\0" as *const u8 as *const std::os::raw::c_char,
        version: 3,
        request_count: 1,
        requests: unsafe { &tertiary_requests as *const _ },
        event_count: 0,
        events: NULLPTR as *const wayland_commons::sys::common::wl_message,
    };