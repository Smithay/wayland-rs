//
// This file was auto-generated, do not edit directly
//

/*
This is an example copyright.
    It contains several lines.
    AS WELL AS ALL CAPS TEXT.
*/

pub mod wl_foo {
    //! Interface for fooing
    //!
    //! This is the dedicated interface for doing foos over any
    //! kind of other foos.
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

    pub trait Handler {
        /// foo numbers
        ///
        /// This request will foo a number and a string.
        fn foo_it(&mut self, evqh: &mut EventLoopHandle, client: &Client,  resource: &WlFoo, number: i32, text: String) {}
        /// create a bar
        ///
        /// Create a bar which will do its bar job.
        fn create_bar(&mut self, evqh: &mut EventLoopHandle, client: &Client,  resource: &WlFoo, id: super::wl_bar::WlBar) {}
        #[doc(hidden)]
        unsafe fn __message(&mut self, evq: &mut EventLoopHandle, client: &Client, proxy: &WlFoo, opcode: u32, args: *const wl_argument) -> Result<(),()> {
            match opcode {
                0 => {
                    let number = {*(args.offset(0) as *const i32)};
                    let text = {String::from_utf8_lossy(CStr::from_ptr(*(args.offset(1) as *const *const _)).to_bytes()).into_owned()};
                    self.foo_it(evq, client, proxy, number, text);
                },
                1 => {
                    let id = {Resource::from_ptr_new(ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_resource_create, client.ptr(), <super::wl_bar::WlBar as Resource>::interface_ptr(), proxy.version(), *(args.offset(0) as *const u32)))};
                    self.create_bar(evq, client, proxy, id);
                },
                _ => return Err(())
            }
            Ok(())
        }
    }

    impl WlFoo {
    }
}
pub mod wl_bar {
    //! Interface for bars
    //!
    //! This interface allows you to bar your foos.
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

pub mod wl_callback {
    //! callback object
    //!
    //! This object has a special behavior regarding its destructor.
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

    pub struct WlCallback {
        ptr: *mut wl_resource,
        data: Option<Arc<(AtomicBool, AtomicPtr<()>)>>
    }

    unsafe impl Send for WlCallback {}
    unsafe impl Sync for WlCallback {}
    impl Resource for WlCallback {
        fn ptr(&self) -> *mut wl_resource { self.ptr }

        unsafe fn from_ptr_new(ptr: *mut wl_resource) -> WlCallback {
            let data = Box::into_raw(Box::new((
                ptr::null_mut::<c_void>(),
                ptr::null_mut::<c_void>(),
                Arc::new((AtomicBool::new(true), AtomicPtr::new(ptr::null_mut())))
            )));
            ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_resource_set_user_data, ptr, data as *mut c_void);
            WlCallback { ptr: ptr, data: Some((&*data).2.clone()) }
        }
        unsafe fn from_ptr_initialized(ptr: *mut wl_resource) -> WlCallback {

            let rust_managed = ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_resource_instance_of,
                ptr, Self::interface_ptr(), &RUST_MANAGED as *const _ as *const _
            ) != 0;

            if rust_managed {
                let data = ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_resource_get_user_data, ptr) as *mut (*mut c_void, *mut c_void, Arc<(AtomicBool, AtomicPtr<()>)>);
                WlCallback { ptr: ptr, data: Some((&*data).2.clone()) }
            } else {
                WlCallback { ptr: ptr, data: Option::None }
            }
        }

        fn interface_ptr() -> *const wl_interface { unsafe { &wl_callback_interface } }
        fn interface_name() -> &'static str { "wl_callback"  }
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

        fn equals(&self, other: &WlCallback) -> bool {
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
    const WL_CALLBACK_DONE: u32 = 0;
    impl WlCallback {
        /// done event
        ///
        /// This event is actually a destructor, but the protocol XML has no wait of specifying it.
        /// As such, the scanner should consider wl_callback.done as a special case.
        ///
        /// This is a destructor, you cannot send events to this object once this method is called.
        pub fn done(&self, callback_data: u32) ->EventResult<()> {
            if self.status() == Liveness::Dead { return EventResult::Destroyed }
            unsafe { ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_resource_post_event, self.ptr(), WL_CALLBACK_DONE, callback_data) };

            if let Some(ref data) = self.data {
                data.0.store(false, ::std::sync::atomic::Ordering::SeqCst);
            }
            unsafe { ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_resource_destroy, self.ptr()); }
            EventResult::Sent(())
        }
    }
}

