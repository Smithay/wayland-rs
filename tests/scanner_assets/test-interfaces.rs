pub static WL_DISPLAY_INTERFACE: wayland_commons::Interface = wayland_commons::Interface {
    name: "wl_display",
    version: 1u32,
    requests: &[
        wayland_commons::MessageDesc {
            name: "sync",
            signature: &[wayland_commons::ArgumentType::NewId],
            since: 1u32,
            is_destructor: false,
            child_interface: Some(&WL_CALLBACK_INTERFACE),
            arg_interfaces: &[],
        },
        wayland_commons::MessageDesc {
            name: "get_registry",
            signature: &[wayland_commons::ArgumentType::NewId],
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
                wayland_commons::ArgumentType::Object,
                wayland_commons::ArgumentType::Uint,
                wayland_commons::ArgumentType::Str,
            ],
            since: 1u32,
            is_destructor: false,
            child_interface: None,
            arg_interfaces: &[&wayland_commons::core_interfaces::ANONYMOUS_INTERFACE],
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
};
pub static WL_REGISTRY_INTERFACE: wayland_commons::Interface = wayland_commons::Interface {
    name: "wl_registry",
    version: 1u32,
    requests: &[wayland_commons::MessageDesc {
        name: "bind",
        signature: &[
            wayland_commons::ArgumentType::Uint,
            wayland_commons::ArgumentType::Str,
            wayland_commons::ArgumentType::Uint,
            wayland_commons::ArgumentType::NewId,
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
                wayland_commons::ArgumentType::Str,
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
                wayland_commons::ArgumentType::Array,
                wayland_commons::ArgumentType::Str,
                wayland_commons::ArgumentType::Fd,
            ],
            since: 1u32,
            is_destructor: false,
            child_interface: None,
            arg_interfaces: &[],
        },
        wayland_commons::MessageDesc {
            name: "get_secondary",
            signature: &[wayland_commons::ArgumentType::NewId],
            since: 2u32,
            is_destructor: false,
            child_interface: Some(&SECONDARY_INTERFACE),
            arg_interfaces: &[],
        },
        wayland_commons::MessageDesc {
            name: "get_tertiary",
            signature: &[wayland_commons::ArgumentType::NewId],
            since: 3u32,
            is_destructor: false,
            child_interface: Some(&TERTIARY_INTERFACE),
            arg_interfaces: &[],
        },
        wayland_commons::MessageDesc {
            name: "link",
            signature: &[
                wayland_commons::ArgumentType::Object,
                wayland_commons::ArgumentType::Object,
                wayland_commons::ArgumentType::Uint,
            ],
            since: 3u32,
            is_destructor: false,
            child_interface: None,
            arg_interfaces: &[&SECONDARY_INTERFACE, &TERTIARY_INTERFACE],
        },
    ],
    events: &[],
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
};