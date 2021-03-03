pub static WL_DISPLAY_INTERFACE: wayland_commons::Interface = wayland_commons::Interface {
    name: "wl_display",
    version: 1u32,
    requests: &[],
    events: &[],
};
pub static WL_REGISTRY_INTERFACE: wayland_commons::Interface = wayland_commons::Interface {
    name: "wl_registry",
    version: 1u32,
    requests: &[wayland_commons::MessageDesc {
        name: "bind",
        signature: &[
            wayland_commons::ArgumentType::Uint,
            wayland_commons::ArgumentType::NewId
        ],
        since: 1u32,
        is_destructor: false,
        child_interface: None,
        arg_interfaces: &[],
    }],
    events: &[],
};
pub static WL_CALLBACK_INTERFACE: wayland_commons::Interface = wayland_commons::Interface {
    name: "wl_callback",
    version: 1u32,
    requests: &[],
    events: &[wayland_commons::MessageDesc {
        name: "done",
        signature: &[wayland_commons::ArgumentType::Uint],
        since: 1u32,
        is_destructor: false,
        child_interface: None,
        arg_interfaces: &[],
    }],
};