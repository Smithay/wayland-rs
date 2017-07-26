//
// This file was auto-generated, do not edit directly
//

/*
This is an example copyright.
  It contains several lines.
  AS WELL AS ALL CAPS TEXT.
*/

pub mod wl_foo {
    use super::Client;
    use super::EventLoopHandle;
    use super::Resource;
    use super::EventResult;
    use super::Liveness;
    use super::interfaces::*;
    use wayland_sys::common::*;
    use std::any::Any;
    use std::ffi::{CString,CStr};
    use std::os::raw::c_void;
    use std::ptr;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};
    use wayland_sys::RUST_MANAGED;
    use wayland_sys::server::*;

    pub struct WlFoo {
        ptr: *mut wl_resource,
        data: Option<Arc<(AtomicBool, AtomicPtr<()>)>>
    }

    unsafe impl Send for WlFoo {}
    unsafe impl Sync for WlFoo {}

    impl Resource for WlFoo {
        fn ptr(&self) -> *mut wl_resource { self.ptr }

        unsafe fn from_ptr_new(ptr: *mut wl_resource) -> WlFoo {
            let data = Box::into_raw(Box::new((
                ptr::null_mut::<c_void>(),
                ptr::null_mut::<c_void>(),
                Arc::new((AtomicBool::new(true), AtomicPtr::new(ptr::null_mut())))
            )));
            ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_resource_set_user_data, ptr, data as *mut c_void);
            WlFoo { ptr: ptr, data: Some((&*data).2.clone()) }
        }
        unsafe fn from_ptr_initialized(ptr: *mut wl_resource) -> WlFoo {

            let rust_managed = ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_resource_instance_of,
                ptr, Self::interface_ptr(), &RUST_MANAGED as *const _ as *const _
            ) != 0;


            if rust_managed {
                let data = ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_resource_get_user_data, ptr) as *mut (*mut c_void, *mut c_void, Arc<(AtomicBool, AtomicPtr<()>)>);
                WlFoo { ptr: ptr, data: Some((&*data).2.clone()) }
            } else {
                WlFoo { ptr: ptr, data: Option::None }
            }
        }

        fn interface_ptr() -> *const wl_interface { unsafe { &wl_foo_interface } }
        fn interface_name() -> &'static str { "wl_foo"  }
        fn supported_version() -> u32 { 3 }
        fn version(&self) -> i32 { unsafe { ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_resource_get_version, self.ptr()) } }

        fn status(&self) -> Liveness {
            if let Some(ref data) = self.data {
                if data.0.load(Ordering::SeqCst) {
                    Liveness::Alive
                } else {
                    Liveness::Dead
                }
            } else {
                Liveness::Unmanaged
            }
        }
        fn equals(&self, other: &WlFoo) -> bool {
            self.status() != Liveness::Dead && other.status() != Liveness::Dead && self.ptr == other.ptr
        }

        fn set_user_data(&self, ptr: *mut ()) {
            if let Some(ref data) = self.data {
                data.1.store(ptr, Ordering::SeqCst);
            }
        }
        fn get_user_data(&self) -> *mut () {
            if let Some(ref data) = self.data {
                data.1.load(Ordering::SeqCst)
            } else {
                ::std::ptr::null_mut()
            }
        }
    }
    impl WlFoo {
    }
}
pub mod wl_bar {
    use super::Client;
    use super::EventLoopHandle;
    use super::Resource;
    use super::EventResult;

    use super::Liveness;
    use super::interfaces::*;
    use wayland_sys::common::*;
    use std::any::Any;
    use std::ffi::{CString,CStr};
    use std::os::raw::c_void;
    use std::ptr;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};
    use wayland_sys::RUST_MANAGED;
    use wayland_sys::server::*;

    pub struct WlBar {
        ptr: *mut wl_resource,
        data: Option<Arc<(AtomicBool, AtomicPtr<()>)>>
    }

    unsafe impl Send for WlBar {}
    unsafe impl Sync for WlBar {}

    impl Resource for WlBar {
        fn ptr(&self) -> *mut wl_resource { self.ptr }

        unsafe fn from_ptr_new(ptr: *mut wl_resource) -> WlBar {
            let data = Box::into_raw(Box::new((
                ptr::null_mut::<c_void>(),
                ptr::null_mut::<c_void>(),
                Arc::new((AtomicBool::new(true), AtomicPtr::new(ptr::null_mut())))
            )));
            ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_resource_set_user_data, ptr, data as *mut c_void);
            WlBar { ptr: ptr, data: Some((&*data).2.clone()) }
        }
        unsafe fn from_ptr_initialized(ptr: *mut wl_resource) -> WlBar {

            let rust_managed = ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_resource_instance_of,
                ptr, Self::interface_ptr(), &RUST_MANAGED as *const _ as *const _
            ) != 0;


            if rust_managed {
                let data = ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_resource_get_user_data, ptr) as *mut (*mut c_void, *mut c_void, Arc<(AtomicBool, AtomicPtr<()>)>);
                WlBar { ptr: ptr, data: Some((&*data).2.clone()) }
            } else {
                WlBar { ptr: ptr, data: Option::None }
            }
        }

        fn interface_ptr() -> *const wl_interface { unsafe { &wl_bar_interface } }
        fn interface_name() -> &'static str { "wl_bar"  }
        fn supported_version() -> u32 { 1 }
        fn version(&self) -> i32 { unsafe { ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_resource_get_version, self.ptr()) } }

        fn status(&self) -> Liveness {
            if let Some(ref data) = self.data {
                if data.0.load(Ordering::SeqCst) {
                    Liveness::Alive
                } else {
                    Liveness::Dead
                }
            } else {
                Liveness::Unmanaged
            }
        }
        fn equals(&self, other: &WlBar) -> bool {
            self.status() != Liveness::Dead && other.status() != Liveness::Dead && self.ptr == other.ptr
        }

        fn set_user_data(&self, ptr: *mut ()) {
            if let Some(ref data) = self.data {
                data.1.store(ptr, Ordering::SeqCst);
            }
        }
        fn get_user_data(&self) -> *mut () {
            if let Some(ref data) = self.data {
                data.1.load(Ordering::SeqCst)
            } else {
                ::std::ptr::null_mut()
            }
        }
    }
    impl WlBar {
    }
}

