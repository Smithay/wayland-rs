//! Bindings to the client library `libwayland-server.so`
//!
//! The generated handle is named `WAYLAND_SERVER_HANDLE`

#![cfg_attr(rustfmt, rustfmt_skip)]

use super::common::*;
use libc::{gid_t, pid_t, uid_t};
use std::os::raw::{c_char, c_int, c_void};

pub enum wl_client {}
pub enum wl_display {}
pub enum wl_event_loop {}
pub enum wl_event_source {}
pub enum wl_global {}
pub enum wl_resource {}
pub enum wl_shm_buffer {}

pub type wl_event_loop_fd_func_t = unsafe extern "C" fn(c_int, u32, *mut c_void) -> c_int;
pub type wl_event_loop_timer_func_t = unsafe extern "C" fn(*mut c_void) -> c_int;
pub type wl_event_loop_signal_func_t = unsafe extern "C" fn(c_int, *mut c_void) -> c_int;
pub type wl_event_loop_idle_func_t = unsafe extern "C" fn(*mut c_void) -> ();
pub type wl_global_bind_func_t = unsafe extern "C" fn(*mut wl_client, *mut c_void, u32, u32) -> ();
pub type wl_notify_func_t = unsafe extern "C" fn(*mut wl_listener, *mut c_void) -> ();
pub type wl_resource_destroy_func_t = unsafe extern "C" fn(*mut wl_resource) -> ();
pub type wl_display_global_filter_func_t = unsafe extern "C" fn(*const wl_client, *const wl_global, *mut c_void) -> bool;

#[repr(C)]
pub struct wl_listener {
    pub link: wl_list,
    pub notify: wl_notify_func_t,
}

#[repr(C)]
pub struct wl_signal {
    pub listener_list: wl_list,
}

external_library!(WaylandServer, "wayland-server",
    functions:
    // wl_client
        fn wl_client_flush(*mut wl_client) -> (),
        fn wl_client_destroy(*mut wl_client) -> (),
        fn wl_client_get_display(*mut wl_client) -> *mut wl_display,
        fn wl_client_get_credentials(*mut wl_client, *mut pid_t, *mut uid_t, *mut gid_t) -> (),
        fn wl_client_get_object(*mut wl_client, u32) -> *mut wl_resource,
        fn wl_client_add_destroy_listener(*mut wl_client, *mut wl_listener) -> (),
        fn wl_client_get_destroy_listener(*mut wl_client, wl_notify_func_t) -> *mut wl_listener,
        fn wl_client_post_no_memory(*mut wl_client) -> (),
        fn wl_resource_create(*mut wl_client, *const wl_interface, c_int, u32) -> *mut wl_resource,
    // wl_display
        fn wl_client_create(*mut wl_display, c_int) -> *mut wl_client,
        fn wl_display_create() -> *mut wl_display,
        fn wl_display_destroy(*mut wl_display) -> (),
        fn wl_display_destroy_clients(*mut wl_display) -> (),
        fn wl_display_get_serial(*mut wl_display) -> u32,
        fn wl_display_next_serial(*mut wl_display) -> u32,
        fn wl_display_add_socket(*mut wl_display, *const c_char) -> c_int,
        fn wl_display_add_socket_auto(*mut wl_display) -> *const c_char,
        fn wl_display_add_socket_fd(*mut wl_display, c_int) -> c_int,
        fn wl_display_add_shm_format(*mut wl_display, u32) -> *mut u32,
        fn wl_display_get_event_loop(*mut wl_display) -> *mut wl_event_loop,
        fn wl_display_terminate(*mut wl_display) -> (),
        fn wl_display_run(*mut wl_display) -> (),
        fn wl_display_flush_clients(*mut wl_display) -> (),
        fn wl_display_add_destroy_listener(*mut wl_display, *mut wl_listener) -> (),
        fn wl_display_get_destroy_listener(*mut wl_display, wl_notify_func_t) -> *mut wl_listener,
        fn wl_global_create(*mut wl_display, *const wl_interface, c_int, *mut c_void, wl_global_bind_func_t) -> *mut wl_global,
        fn wl_display_init_shm(*mut wl_display) -> c_int,
        fn wl_display_add_client_created_listener(*mut wl_display, *mut wl_listener) -> (),
        fn wl_display_set_global_filter(*mut wl_display, wl_display_global_filter_func_t, *mut c_void) -> (),
    // wl_event_loop
        fn wl_event_loop_create() -> *mut wl_event_loop,
        fn wl_event_loop_destroy(*mut wl_event_loop) -> (),
        fn wl_event_loop_add_fd(*mut wl_event_loop, c_int, u32, wl_event_loop_fd_func_t, *mut c_void) -> *mut wl_event_source,
        fn wl_event_loop_add_timer(*mut wl_event_loop, wl_event_loop_timer_func_t, *mut c_void) -> *mut wl_event_source,
        fn wl_event_loop_add_signal(*mut wl_event_loop, c_int, wl_event_loop_signal_func_t, *mut c_void) -> *mut wl_event_source,
        fn wl_event_loop_dispatch(*mut wl_event_loop, c_int) -> c_int,
        fn wl_event_loop_dispatch_idle(*mut wl_event_loop) -> (),
        fn wl_event_loop_add_idle(*mut wl_event_loop, wl_event_loop_idle_func_t, *mut c_void) -> *mut wl_event_source,
        fn wl_event_loop_get_fd(*mut wl_event_loop) -> c_int,
        fn wl_event_loop_add_destroy_listener(*mut wl_event_loop, *mut wl_listener) -> (),
        fn wl_event_loop_get_destroy_listener(*mut wl_event_loop, wl_notify_func_t) -> *mut wl_listener,
    // wl_event_source
        fn wl_event_source_fd_update(*mut wl_event_source, u32) -> c_int,
        fn wl_event_source_timer_update(*mut wl_event_source, c_int) -> c_int,
        fn wl_event_source_remove(*mut wl_event_source) -> c_int,
        fn wl_event_source_check(*mut wl_event_source) -> (),
    // wl_global
        fn wl_global_destroy(*mut wl_global) -> (),
        fn wl_global_get_user_data(*const wl_global) -> *mut c_void,
    // wl_resource
        fn wl_resource_post_event_array(*mut wl_resource, u32, *mut wl_argument) -> (),
        fn wl_resource_queue_event_array(*mut wl_resource, u32, *mut wl_argument) -> (),
        fn wl_resource_post_no_memory(*mut wl_resource) -> (),
        fn wl_resource_set_implementation(*mut wl_resource, *const c_void, *mut c_void, Option<wl_resource_destroy_func_t>) -> (),
        fn wl_resource_set_dispatcher(*mut wl_resource, wl_dispatcher_func_t, *const c_void, *mut c_void, Option<wl_resource_destroy_func_t>) -> (),
        fn wl_resource_destroy(*mut wl_resource) -> (),
        fn wl_resource_get_client(*mut wl_resource) -> *mut wl_client,
        fn wl_resource_get_id(*mut wl_resource) -> u32,
        fn wl_resource_get_link(*mut wl_resource) -> *mut wl_list,
        fn wl_resource_from_link(*mut wl_list) -> *mut wl_resource,
        fn wl_resource_find_for_client(*mut wl_list, *mut wl_client) -> (),
        fn wl_resource_set_user_data(*mut wl_resource, *mut c_void) -> (),
        fn wl_resource_get_user_data(*mut wl_resource) -> *mut c_void,
        fn wl_resource_get_version(*mut wl_resource) -> c_int,
        fn wl_resource_set_destructor(*mut wl_resource, Option<wl_resource_destroy_func_t>) -> (),
        fn wl_resource_instance_of(*mut wl_resource, *const wl_interface, *const c_void) -> c_int,
        fn wl_resource_add_destroy_listener(*mut wl_resource, wl_notify_func_t) -> (),
        fn wl_resource_get_destroy_listener(*mut wl_resource,wl_notify_func_t) -> *mut wl_listener,
    // wl_shm
        fn wl_shm_buffer_begin_access(*mut wl_shm_buffer) -> (),
        fn wl_shm_buffer_end_access(*mut wl_shm_buffer) -> (),
        fn wl_shm_buffer_get(*mut wl_resource) -> *mut wl_shm_buffer,
        fn wl_shm_buffer_get_data(*mut wl_shm_buffer) -> *mut c_void,
        fn wl_shm_buffer_get_stride(*mut wl_shm_buffer) -> i32,
        fn wl_shm_buffer_get_format(*mut wl_shm_buffer) -> u32,
        fn wl_shm_buffer_get_width(*mut wl_shm_buffer) -> i32,
        fn wl_shm_buffer_get_height(*mut wl_shm_buffer) -> i32,
    // wl_log
        fn wl_log_set_handler_server(wl_log_func_t) -> (),
    // wl_list
        fn wl_list_init(*mut wl_list) -> (),
        fn wl_list_insert(*mut wl_list, *mut wl_list) -> (),
        fn wl_list_remove(*mut wl_list) -> (),
        fn wl_list_length(*const wl_list) -> c_int,
        fn wl_list_empty(*const wl_list) -> c_int,
        fn wl_list_insert_list(*mut wl_list,*mut wl_list) -> (),

    // arrays
        fn wl_array_init(*mut wl_array) -> (),
        fn wl_array_release(*mut wl_array) -> (),
        fn wl_array_add(*mut wl_array,usize) -> (),
        fn wl_array_copy(*mut wl_array, *mut wl_array) -> (),
    varargs:
        fn wl_resource_post_event(*mut wl_resource, u32) -> (),
        fn wl_resource_queue_event(*mut wl_resource, u32) -> (),
        fn wl_resource_post_error(*mut wl_resource, u32, *const c_char) -> (),
);

#[cfg(feature = "dlopen")]
lazy_static!(
    pub static ref WAYLAND_SERVER_OPTION: Option<WaylandServer> = {
        // This is a workaround for Ubuntu 17.04, which doesn't have a bare symlink
        // for libwayland-server.so but does have it with the version numbers for
        // whatever reason.
        //
        // We could do some trickery with str slices but that is more trouble
        // than its worth
        let versions = ["libwayland-server.so",
                        "libwayland-server.so.0"];
        for ver in &versions {
            match WaylandServer::open(ver) {
                Ok(h) => return Some(h),
                Err(::dlib::DlError::NotFound) => continue,
                Err(::dlib::DlError::MissingSymbol(s)) => {
                    if ::std::env::var_os("WAYLAND_RS_DEBUG").is_some() {
                        // only print debug messages if WAYLAND_RS_DEBUG is set
                        eprintln!("[wayland-server] Found library {} cannot be used: symbol {} is missing.", ver, s);
                    }
                    return None;
                }
            }
        }
        None
    };
    pub static ref WAYLAND_SERVER_HANDLE: &'static WaylandServer = {
        WAYLAND_SERVER_OPTION.as_ref().expect("Library libwayland-server.so could not be loaded.")
    };
);

#[cfg(not(feature = "dlopen"))]
pub fn is_lib_available() -> bool {
    true
}
#[cfg(feature = "dlopen")]
pub fn is_lib_available() -> bool {
    WAYLAND_SERVER_OPTION.is_some()
}

pub mod signal {
    #[cfg(not(feature = "dlopen"))]
    use super::{wl_list_init, wl_list_insert};
    use super::{wl_listener, wl_notify_func_t, wl_signal};
    #[cfg(feature = "dlopen")]
    use super::WAYLAND_SERVER_HANDLE as WSH;
    use common::wl_list;
    use std::os::raw::c_void;
    use std::ptr;

    // TODO: Is this really not UB ?
    macro_rules! offset_of(
        ($ty:ty, $field:ident) => {
            &(*(0 as *const $ty)).$field as *const _ as usize
        }
    );

    macro_rules! container_of(
        ($ptr: expr, $container: ty, $field: ident) => {
            ($ptr as *mut u8).offset(-(offset_of!($container, $field) as isize)) as *mut $container
        }
    );

    macro_rules! list_for_each(
        ($pos: ident, $head:expr, $container: ty, $field: ident, $action: block) => {
            let mut $pos = container_of!((*$head).next, $container, $field);
            while &mut (*$pos).$field as *mut _ != $head {
                $action;
                $pos = container_of!((*$pos).$field.next, $container, $field);
            }
        }
    );

    macro_rules! list_for_each_safe(
        ($pos: ident, $head: expr, $container: ty, $field: ident, $action: block) => {
            let mut $pos = container_of!((*$head).next, $container, $field);
            let mut tmp = container_of!((*$pos).$field.next, $container, $field);
            while &mut (*$pos).$field as *mut _ != $head {
                $action;
                $pos = tmp;
                tmp = container_of!((*$pos).$field.next, $container, $field);
            }
        }
    );

    pub unsafe fn wl_signal_init(signal: *mut wl_signal) {
        ffi_dispatch!(WSH, wl_list_init, &mut (*signal).listener_list);
    }

    pub unsafe fn wl_signal_add(signal: *mut wl_signal, listener: *mut wl_listener) {
        ffi_dispatch!(
            WSH,
            wl_list_insert,
            (*signal).listener_list.prev,
            &mut (*listener).link
        )
    }

    pub unsafe fn wl_signal_get(signal: *mut wl_signal, notify: wl_notify_func_t) -> *mut wl_listener {
        list_for_each!(
            l,
            &mut (*signal).listener_list as *mut wl_list,
            wl_listener,
            link,
            {
                if (*l).notify == notify {
                    return l;
                }
            }
        );

        ptr::null_mut()
    }

    pub unsafe fn wl_signal_emit(signal: *mut wl_signal, data: *mut c_void) {
        list_for_each_safe!(
            l,
            &mut (*signal).listener_list as *mut wl_list,
            wl_listener,
            link,
            {
                ((*l).notify)(l, data);
            }
        );
    }

    #[repr(C)]
    struct ListenerWithUserData {
        listener: wl_listener,
        user_data: *mut c_void
    }

    pub fn rust_listener_create(notify: wl_notify_func_t) -> *mut wl_listener {
        let data = Box::into_raw(Box::new(ListenerWithUserData {
            listener: wl_listener {
                link: wl_list {
                    prev: ptr::null_mut(),
                    next: ptr::null_mut()
                },
                notify
            },
            user_data: ptr::null_mut()
        }));

        unsafe { &mut (*data).listener as *mut wl_listener }
    }

    pub unsafe fn rust_listener_get_user_data(listener: *mut wl_listener) -> *mut c_void {
        let data = container_of!(listener, ListenerWithUserData, listener);
        (*data).user_data
    }

    pub unsafe fn rust_listener_set_user_data(listener: *mut wl_listener, user_data: *mut c_void) {
        let data = container_of!(listener, ListenerWithUserData, listener);
        (*data).user_data = user_data
    }

    pub unsafe fn rust_listener_destroy(listener: *mut wl_listener) {
        let data = container_of!(listener, ListenerWithUserData, listener);
        let _ = Box::from_raw(data);
    }
}
