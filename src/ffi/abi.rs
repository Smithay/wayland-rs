use libc::{c_int, c_char, c_void, int32_t, uint32_t, size_t};

#[repr(C)] pub struct wl_argument;

#[repr(C)]
pub struct wl_array {
    pub size: size_t,
    pub alloc: size_t,
    pub data: *mut c_void
}

#[repr(C)] pub struct wl_display;
#[repr(C)] pub struct wl_event_queue;

/// Type representing an interface in the `libwayland-client.so` ABI.
///
/// This type allows you to manually bind to a wayland protocol extension
/// not (yet?) supported by this library, via the `FFI` and `Bind` traits.
#[repr(C)]
pub struct wl_interface {
    pub name: *const char,
    pub version: c_int,
    pub method_count: c_int,
    pub methods: *const wl_message,
    pub event_count: c_int,
    pub events: *const wl_message,
}

#[repr(C)]
pub struct wl_list {
    pub prev: *mut wl_list,
    pub next: *mut wl_list,
}

/// Type representing a message in the `libwayland-client.so` ABI.
#[repr(C)]
pub struct wl_message {
    pub name: *const c_char,
    pub signature: *const c_char,
    pub types: *const *mut wl_interface,
}

#[repr(C)] pub struct wl_proxy;

#[repr(C)] pub type wl_dispatcher_func_t = extern fn(*const c_void, 
                                                     *mut c_void,
                                                     uint32_t,
                                                     *const wl_message,
                                                     *mut wl_argument
                                                    );
#[repr(C)] pub type wl_log_func_t = extern fn(_: *const c_char, ...);

#[repr(C)] pub type wl_fixed_t = int32_t;

pub fn wl_fixed_to_double(f: wl_fixed_t) -> f64 {
    f as f64 / 256.
}

pub fn wl_fixed_from_double(d: f64) -> wl_fixed_t {
    (d * 256.) as i32
}

external_library!(WaylandClient, "wayland-client",
    statics:
        wl_buffer_interface: wl_interface,
        wl_callback_interface: wl_interface,
        wl_compositor_interface: wl_interface,
        wl_data_device_interface: wl_interface,
        wl_data_device_manager_interface: wl_interface,
        wl_data_offer_interface: wl_interface,
        wl_data_source_interface: wl_interface,
        wl_display_interface: wl_interface,
        wl_keyboard_interface: wl_interface,
        wl_output_interface: wl_interface,
        wl_pointer_interface: wl_interface,
        wl_region_interface: wl_interface,
        wl_registry_interface: wl_interface,
        wl_seat_interface: wl_interface,
        wl_shell_interface: wl_interface,
        wl_shell_surface_interface: wl_interface,
        wl_shm_interface: wl_interface,
        wl_shm_pool_interface: wl_interface,
        wl_subcompositor_interface: wl_interface,
        wl_subsurface_interface: wl_interface,
        wl_surface_interface: wl_interface,
        wl_touch_interface: wl_interface

    functions:
    // display creation and destruction
        fn wl_display_connect_to_fd(c_int) -> *mut wl_display,
        fn wl_display_connect(*const c_char) -> *mut wl_display,
        fn wl_display_disconnect(*mut wl_display) -> (),
        fn wl_display_get_fd(*mut wl_display) -> c_int,
    // display events handling
        fn wl_display_roundtrip(*mut wl_display) -> c_int,
        fn wl_display_read_events(*mut wl_display) -> c_int,
        fn wl_display_prepare_read(*mut wl_display) -> c_int,
        fn wl_display_cancel_read(*mut wl_display) -> (),
        fn wl_display_dispatch(*mut wl_display) -> c_int,
        fn wl_display_dispatch_pending(*mut wl_display) -> c_int,
    // error handling
        fn wl_display_get_error(*mut wl_display) -> c_int,
        fn wl_display_get_protocol_error(*mut wl_display, *mut *mut wl_interface, *mut uint32_t) -> uint32_t,
    // requests handling
        fn wl_display_flush(*mut wl_display) -> c_int,

    // event queues
        fn wl_event_queue_destroy(*mut wl_event_queue) -> (),
        fn wl_display_create_queue(*mut wl_display) -> *mut wl_event_queue,
        fn wl_display_roundtrip_queue(*mut wl_display, *mut wl_event_queue) -> c_int,
        fn wl_display_prepare_read_queue(*mut wl_display, *mut wl_event_queue) -> c_int,
        fn wl_display_dispatch_queue(*mut wl_display, *mut wl_event_queue) -> c_int,
        fn wl_display_dispatch_queue_pending(*mut wl_display, *mut wl_event_queue) -> c_int,

    // proxys
        fn wl_proxy_create(*mut wl_proxy, *const wl_interface) -> *mut wl_proxy,
        fn wl_proxy_destroy(*mut wl_proxy) -> (),
        fn wl_proxy_add_listener(*mut wl_proxy, *mut extern fn(), *mut c_void) -> c_int,
        fn wl_proxy_get_listener(*mut wl_proxy) -> *const c_void,
        fn wl_proxy_add_dispatcher(*mut wl_proxy, wl_dispatcher_func_t, *const c_void, *mut c_void) -> c_int,
        fn wl_proxy_marshal_array_constructor(*mut wl_proxy, uint32_t, *mut wl_argument, *const wl_interface) -> c_int,

        fn wl_proxy_marshal_array(*mut wl_proxy, uint32_t, *mut wl_argument ) -> (),
        fn wl_proxy_set_user_data(*mut wl_proxy, *mut c_void) -> (),
        fn wl_proxy_get_user_data(*mut wl_proxy) -> *mut c_void,
        fn wl_proxy_get_id(*mut wl_proxy) -> uint32_t,
        fn wl_proxy_get_class(*mut wl_proxy) -> *const c_char,
        fn wl_proxy_set_queue(*mut wl_proxy, *mut wl_event_queue) -> (),

    // log
        fn wl_log_set_handler_client(wl_log_func_t) -> (),
    // wl_log(fmt: *const c_char, ...) -> (),

    // lists
        fn wl_list_init(*mut wl_list) -> (),
        fn wl_list_insert(*mut wl_list, *mut wl_list) -> (),
        fn wl_list_remove(*mut wl_list) -> (),
        fn wl_list_length(*const wl_list) -> c_int,
        fn wl_list_empty(*const wl_list) -> c_int,
        fn wl_list_insert_list(*mut wl_list,*mut wl_list) -> (),

    // arrays
        fn wl_array_init(*mut wl_array) -> (),
        fn wl_array_release(*mut wl_array) -> (),
        fn wl_array_add(*mut wl_array,size_t) -> (),
        fn wl_array_copy(*mut wl_array, *mut wl_array) -> ()

    varargs:
        fn wl_proxy_marshal_constructor(*mut wl_proxy, uint32_t, *const wl_interface ...) -> *mut wl_proxy,
        fn wl_proxy_marshal(*mut wl_proxy, uint32_t ...) -> ()
);

#[cfg(feature = "dlopen")]
lazy_static!(
    pub static ref WAYLAND_CLIENT_OPTION: Option<WaylandClient> = { 
        WaylandClient::open("libwayland-client.so").ok()
    };
    pub static ref WAYLAND_CLIENT_HANDLE: &'static WaylandClient = {
        WAYLAND_CLIENT_OPTION.as_ref().expect("Library libwayland-client.so could not be loaded.")
    };
);

#[cfg(not(feature = "dlopen"))]
#[link(name = "wayland-client")]
extern {

}