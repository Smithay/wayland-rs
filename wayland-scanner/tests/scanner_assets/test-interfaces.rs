static types_null: [Option<&wayland_backend::protocol::CWlInterface>; 6] = [None; 6];
pub static WL_DISPLAY_INTERFACE: wayland_backend::protocol::Interface =
    wayland_backend::protocol::Interface {
        name: "wl_display",
        version: 1u32,
        requests: &[
            wayland_backend::protocol::MessageDesc {
                name: "sync",
                signature: &[wayland_backend::protocol::ArgumentType::NewId],
                since: 1u32,
                is_destructor: false,
                child_interface: Some(&WL_CALLBACK_INTERFACE),
                arg_interfaces: &[],
            },
            wayland_backend::protocol::MessageDesc {
                name: "get_registry",
                signature: &[wayland_backend::protocol::ArgumentType::NewId],
                since: 1u32,
                is_destructor: false,
                child_interface: Some(&WL_REGISTRY_INTERFACE),
                arg_interfaces: &[],
            },
        ],
        events: &[
            wayland_backend::protocol::MessageDesc {
                name: "error",
                signature: &[
                    wayland_backend::protocol::ArgumentType::Object(
                        wayland_backend::protocol::AllowNull::No,
                    ),
                    wayland_backend::protocol::ArgumentType::Uint,
                    wayland_backend::protocol::ArgumentType::Str(
                        wayland_backend::protocol::AllowNull::No,
                    ),
                ],
                since: 1u32,
                is_destructor: false,
                child_interface: None,
                arg_interfaces: &[&wayland_backend::protocol::ANONYMOUS_INTERFACE],
            },
            wayland_backend::protocol::MessageDesc {
                name: "delete_id",
                signature: &[wayland_backend::protocol::ArgumentType::Uint],
                since: 1u32,
                is_destructor: false,
                child_interface: None,
                arg_interfaces: &[],
            },
        ],
        c_interface: Some(unsafe { &wl_display_interface }),
    };
static wl_display_requests: [wayland_backend::protocol::CWlMessage; 2] = [
    wayland_backend::protocol::CWlMessage::new(
        unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"sync\0") },
        unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"n\0") },
        &[Some(&wl_callback_interface)],
    ),
    wayland_backend::protocol::CWlMessage::new(
        unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"get_registry\0") },
        unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"n\0") },
        &[Some(&wl_registry_interface)],
    ),
];
static wl_display_events: [wayland_backend::protocol::CWlMessage; 2] = [
    wayland_backend::protocol::CWlMessage::new(
        unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"error\0") },
        unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"ous\0") },
        &types_null,
    ),
    wayland_backend::protocol::CWlMessage::new(
        unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"delete_id\0") },
        unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"u\0") },
        &types_null,
    ),
];
pub static wl_display_interface: wayland_backend::protocol::CWlInterface =
    wayland_backend::protocol::CWlInterface::new(
        unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"wl_display\0") },
        1,
        &wl_display_requests,
        &wl_display_events,
    );
pub static WL_REGISTRY_INTERFACE: wayland_backend::protocol::Interface =
    wayland_backend::protocol::Interface {
        name: "wl_registry",
        version: 1u32,
        requests: &[wayland_backend::protocol::MessageDesc {
            name: "bind",
            signature: &[
                wayland_backend::protocol::ArgumentType::Uint,
                wayland_backend::protocol::ArgumentType::Str(
                    wayland_backend::protocol::AllowNull::No,
                ),
                wayland_backend::protocol::ArgumentType::Uint,
                wayland_backend::protocol::ArgumentType::NewId,
            ],
            since: 1u32,
            is_destructor: false,
            child_interface: None,
            arg_interfaces: &[],
        }],
        events: &[
            wayland_backend::protocol::MessageDesc {
                name: "global",
                signature: &[
                    wayland_backend::protocol::ArgumentType::Uint,
                    wayland_backend::protocol::ArgumentType::Str(
                        wayland_backend::protocol::AllowNull::No,
                    ),
                    wayland_backend::protocol::ArgumentType::Uint,
                ],
                since: 1u32,
                is_destructor: false,
                child_interface: None,
                arg_interfaces: &[],
            },
            wayland_backend::protocol::MessageDesc {
                name: "global_remove",
                signature: &[wayland_backend::protocol::ArgumentType::Uint],
                since: 1u32,
                is_destructor: false,
                child_interface: None,
                arg_interfaces: &[],
            },
        ],
        c_interface: Some(unsafe { &wl_registry_interface }),
    };
static wl_registry_requests: [wayland_backend::protocol::CWlMessage; 1] =
    [wayland_backend::protocol::CWlMessage::new(
        unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"bind\0") },
        unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"usun\0") },
        &types_null,
    )];
static wl_registry_events: [wayland_backend::protocol::CWlMessage; 2] = [
    wayland_backend::protocol::CWlMessage::new(
        unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"global\0") },
        unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"usu\0") },
        &types_null,
    ),
    wayland_backend::protocol::CWlMessage::new(
        unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"global_remove\0") },
        unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"u\0") },
        &types_null,
    ),
];
pub static wl_registry_interface: wayland_backend::protocol::CWlInterface =
    wayland_backend::protocol::CWlInterface::new(
        unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"wl_registry\0") },
        1,
        &wl_registry_requests,
        &wl_registry_events,
    );
pub static WL_CALLBACK_INTERFACE: wayland_backend::protocol::Interface =
    wayland_backend::protocol::Interface {
        name: "wl_callback",
        version: 1u32,
        requests: &[],
        events: &[wayland_backend::protocol::MessageDesc {
            name: "done",
            signature: &[wayland_backend::protocol::ArgumentType::Uint],
            since: 1u32,
            is_destructor: true,
            child_interface: None,
            arg_interfaces: &[],
        }],
        c_interface: Some(unsafe { &wl_callback_interface }),
    };
static wl_callback_events: [wayland_backend::protocol::CWlMessage; 1] =
    [wayland_backend::protocol::CWlMessage::new(
        unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"done\0") },
        unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"u\0") },
        &types_null,
    )];
pub static wl_callback_interface: wayland_backend::protocol::CWlInterface =
    wayland_backend::protocol::CWlInterface::new(
        unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"wl_callback\0") },
        1,
        &[],
        &wl_callback_events,
    );
pub static TEST_GLOBAL_INTERFACE: wayland_backend::protocol::Interface =
    wayland_backend::protocol::Interface {
        name: "test_global",
        version: 5u32,
        requests: &[
            wayland_backend::protocol::MessageDesc {
                name: "many_args",
                signature: &[
                    wayland_backend::protocol::ArgumentType::Uint,
                    wayland_backend::protocol::ArgumentType::Int,
                    wayland_backend::protocol::ArgumentType::Fixed,
                    wayland_backend::protocol::ArgumentType::Array,
                    wayland_backend::protocol::ArgumentType::Str(
                        wayland_backend::protocol::AllowNull::No,
                    ),
                    wayland_backend::protocol::ArgumentType::Fd,
                ],
                since: 1u32,
                is_destructor: false,
                child_interface: None,
                arg_interfaces: &[],
            },
            wayland_backend::protocol::MessageDesc {
                name: "get_secondary",
                signature: &[wayland_backend::protocol::ArgumentType::NewId],
                since: 2u32,
                is_destructor: false,
                child_interface: Some(&SECONDARY_INTERFACE),
                arg_interfaces: &[],
            },
            wayland_backend::protocol::MessageDesc {
                name: "get_tertiary",
                signature: &[wayland_backend::protocol::ArgumentType::NewId],
                since: 3u32,
                is_destructor: false,
                child_interface: Some(&TERTIARY_INTERFACE),
                arg_interfaces: &[],
            },
            wayland_backend::protocol::MessageDesc {
                name: "link",
                signature: &[
                    wayland_backend::protocol::ArgumentType::Object(
                        wayland_backend::protocol::AllowNull::No,
                    ),
                    wayland_backend::protocol::ArgumentType::Object(
                        wayland_backend::protocol::AllowNull::Yes,
                    ),
                    wayland_backend::protocol::ArgumentType::Uint,
                ],
                since: 3u32,
                is_destructor: false,
                child_interface: None,
                arg_interfaces: &[&SECONDARY_INTERFACE, &TERTIARY_INTERFACE],
            },
            wayland_backend::protocol::MessageDesc {
                name: "destroy",
                signature: &[],
                since: 4u32,
                is_destructor: true,
                child_interface: None,
                arg_interfaces: &[],
            },
            wayland_backend::protocol::MessageDesc {
                name: "reverse_link",
                signature: &[
                    wayland_backend::protocol::ArgumentType::Object(
                        wayland_backend::protocol::AllowNull::Yes,
                    ),
                    wayland_backend::protocol::ArgumentType::Object(
                        wayland_backend::protocol::AllowNull::No,
                    ),
                ],
                since: 5u32,
                is_destructor: false,
                child_interface: None,
                arg_interfaces: &[&SECONDARY_INTERFACE, &TERTIARY_INTERFACE],
            },
            wayland_backend::protocol::MessageDesc {
                name: "newid_and_allow_null",
                signature: &[
                    wayland_backend::protocol::ArgumentType::NewId,
                    wayland_backend::protocol::ArgumentType::Object(
                        wayland_backend::protocol::AllowNull::Yes,
                    ),
                    wayland_backend::protocol::ArgumentType::Object(
                        wayland_backend::protocol::AllowNull::No,
                    ),
                ],
                since: 5u32,
                is_destructor: false,
                child_interface: Some(&QUAD_INTERFACE),
                arg_interfaces: &[&SECONDARY_INTERFACE, &TERTIARY_INTERFACE],
            },
        ],
        events: &[
            wayland_backend::protocol::MessageDesc {
                name: "many_args_evt",
                signature: &[
                    wayland_backend::protocol::ArgumentType::Uint,
                    wayland_backend::protocol::ArgumentType::Int,
                    wayland_backend::protocol::ArgumentType::Fixed,
                    wayland_backend::protocol::ArgumentType::Array,
                    wayland_backend::protocol::ArgumentType::Str(
                        wayland_backend::protocol::AllowNull::No,
                    ),
                    wayland_backend::protocol::ArgumentType::Fd,
                ],
                since: 1u32,
                is_destructor: false,
                child_interface: None,
                arg_interfaces: &[],
            },
            wayland_backend::protocol::MessageDesc {
                name: "ack_secondary",
                signature: &[wayland_backend::protocol::ArgumentType::Object(
                    wayland_backend::protocol::AllowNull::No,
                )],
                since: 1u32,
                is_destructor: false,
                child_interface: None,
                arg_interfaces: &[&SECONDARY_INTERFACE],
            },
            wayland_backend::protocol::MessageDesc {
                name: "cycle_quad",
                signature: &[
                    wayland_backend::protocol::ArgumentType::NewId,
                    wayland_backend::protocol::ArgumentType::Object(
                        wayland_backend::protocol::AllowNull::Yes,
                    ),
                ],
                since: 1u32,
                is_destructor: false,
                child_interface: Some(&QUAD_INTERFACE),
                arg_interfaces: &[&QUAD_INTERFACE],
            },
        ],
        c_interface: Some(unsafe { &test_global_interface }),
    };
static test_global_requests: [wayland_backend::protocol::CWlMessage; 7] = [
    wayland_backend::protocol::CWlMessage::new(
        unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"many_args\0") },
        unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"uifash\0") },
        &types_null,
    ),
    wayland_backend::protocol::CWlMessage::new(
        unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"get_secondary\0") },
        unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"2n\0") },
        &[Some(&secondary_interface)],
    ),
    wayland_backend::protocol::CWlMessage::new(
        unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"get_tertiary\0") },
        unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"3n\0") },
        &[Some(&tertiary_interface)],
    ),
    wayland_backend::protocol::CWlMessage::new(
        unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"link\0") },
        unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"3o?ou\0") },
        &[Some(&secondary_interface), Some(&tertiary_interface), None],
    ),
    wayland_backend::protocol::CWlMessage::new(
        unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"destroy\0") },
        unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"4\0") },
        &types_null,
    ),
    wayland_backend::protocol::CWlMessage::new(
        unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"reverse_link\0") },
        unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"5?oo\0") },
        &[Some(&secondary_interface), Some(&tertiary_interface)],
    ),
    wayland_backend::protocol::CWlMessage::new(
        unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"newid_and_allow_null\0") },
        unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"5n?oo\0") },
        &[Some(&quad_interface), Some(&secondary_interface), Some(&tertiary_interface)],
    ),
];
static test_global_events: [wayland_backend::protocol::CWlMessage; 3] = [
    wayland_backend::protocol::CWlMessage::new(
        unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"many_args_evt\0") },
        unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"uifash\0") },
        &types_null,
    ),
    wayland_backend::protocol::CWlMessage::new(
        unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"ack_secondary\0") },
        unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"o\0") },
        &[Some(&secondary_interface)],
    ),
    wayland_backend::protocol::CWlMessage::new(
        unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"cycle_quad\0") },
        unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"n?o\0") },
        &[Some(&quad_interface), Some(&quad_interface)],
    ),
];
pub static test_global_interface: wayland_backend::protocol::CWlInterface =
    wayland_backend::protocol::CWlInterface::new(
        unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"test_global\0") },
        5,
        &test_global_requests,
        &test_global_events,
    );
pub static SECONDARY_INTERFACE: wayland_backend::protocol::Interface =
    wayland_backend::protocol::Interface {
        name: "secondary",
        version: 5u32,
        requests: &[wayland_backend::protocol::MessageDesc {
            name: "destroy",
            signature: &[],
            since: 2u32,
            is_destructor: true,
            child_interface: None,
            arg_interfaces: &[],
        }],
        events: &[],
        c_interface: Some(unsafe { &secondary_interface }),
    };
static secondary_requests: [wayland_backend::protocol::CWlMessage; 1] =
    [wayland_backend::protocol::CWlMessage::new(
        unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"destroy\0") },
        unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"2\0") },
        &types_null,
    )];
pub static secondary_interface: wayland_backend::protocol::CWlInterface =
    wayland_backend::protocol::CWlInterface::new(
        unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"secondary\0") },
        5,
        &secondary_requests,
        &[],
    );
pub static TERTIARY_INTERFACE: wayland_backend::protocol::Interface =
    wayland_backend::protocol::Interface {
        name: "tertiary",
        version: 5u32,
        requests: &[wayland_backend::protocol::MessageDesc {
            name: "destroy",
            signature: &[],
            since: 3u32,
            is_destructor: true,
            child_interface: None,
            arg_interfaces: &[],
        }],
        events: &[],
        c_interface: Some(unsafe { &tertiary_interface }),
    };
static tertiary_requests: [wayland_backend::protocol::CWlMessage; 1] =
    [wayland_backend::protocol::CWlMessage::new(
        unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"destroy\0") },
        unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"3\0") },
        &types_null,
    )];
pub static tertiary_interface: wayland_backend::protocol::CWlInterface =
    wayland_backend::protocol::CWlInterface::new(
        unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"tertiary\0") },
        5,
        &tertiary_requests,
        &[],
    );
pub static QUAD_INTERFACE: wayland_backend::protocol::Interface =
    wayland_backend::protocol::Interface {
        name: "quad",
        version: 5u32,
        requests: &[wayland_backend::protocol::MessageDesc {
            name: "destroy",
            signature: &[],
            since: 3u32,
            is_destructor: true,
            child_interface: None,
            arg_interfaces: &[],
        }],
        events: &[],
        c_interface: Some(unsafe { &quad_interface }),
    };
static quad_requests: [wayland_backend::protocol::CWlMessage; 1] =
    [wayland_backend::protocol::CWlMessage::new(
        unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"destroy\0") },
        unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"3\0") },
        &types_null,
    )];
pub static quad_interface: wayland_backend::protocol::CWlInterface =
    wayland_backend::protocol::CWlInterface::new(
        unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"quad\0") },
        5,
        &quad_requests,
        &[],
    );
