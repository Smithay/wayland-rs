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
    use super::{Liveness, Implementable};
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

    unsafe impl Resource for WlFoo {
        fn ptr(&self) -> *mut wl_resource { self.ptr }

        unsafe fn from_ptr_new(ptr: *mut wl_resource) -> WlFoo {
            let data = Box::into_raw(Box::new((
                ptr::null_mut::<c_void>(),
                Option::None::<Box<Any>>,
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
                let data = ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_resource_get_user_data, ptr) as *mut (*mut c_void, Option<Box<Any>>, Arc<(AtomicBool, AtomicPtr<()>)>);
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
        unsafe fn clone_unchecked(&self) -> WlFoo {
            WlFoo {
                ptr: self.ptr,
                data: self.data.clone()
            }
        }
    }
    unsafe impl<ID: 'static> Implementable<ID> for WlFoo {
        type Implementation = Implementation<ID>;
        #[allow(unused_mut,unused_assignments)]
        unsafe fn __dispatch_msg(&self, client: &Client, opcode: u32, args: *const wl_argument) -> Result<(),()> {

        let data: &mut (*mut EventLoopHandle, Option<Box<Any>>, Arc<(AtomicBool, AtomicPtr<()>)>) =
            &mut *(ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_resource_get_user_data, self.ptr()) as *mut _);
        let evq = &mut *(data.0);
        let mut kill = false;
        {
            let &mut (ref implementation, ref mut idata) = data.1.as_mut().unwrap().downcast_mut::<(Implementation<ID>, ID)>().unwrap();
            match opcode {
                0 => {
                    let number = {*(args.offset(0) as *const i32)};
                    let unumber = {*(args.offset(1) as *const u32)};
                    let text = {String::from_utf8_lossy(CStr::from_ptr(*(args.offset(2) as *const *const _)).to_bytes()).into_owned()};
                    let float = {wl_fixed_to_double(*(args.offset(3) as *const i32))};
                    let file = {*(args.offset(4) as *const i32)};
                    (implementation.foo_it)(evq, idata, client, self, number, unumber, text, float, file);
                },
                1 => {
                    let id = {Resource::from_ptr_new(ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_resource_create, client.ptr(), <super::wl_bar::WlBar as Resource>::interface_ptr(), self.version(), *(args.offset(0) as *const u32)))};
                    (implementation.create_bar)(evq, idata, client, self, id);
                },
                _ => return Err(())
            }
        }

        if kill {
            let _impl = data.1.take();
            ::std::mem::drop(_impl);
        }
            Ok(())
        }
    }

    /// Possible cake kinds
    ///
    /// List of the possible kind of cake supported by the protocol.
    #[repr(u32)]
    #[derive(Copy,Clone,Debug,PartialEq)]
    pub enum CakeKind {
        Basic = 0,
        Spicy = 1,
        Fruity = 2,
    }
    impl CakeKind {
        pub fn from_raw(n: u32) -> Option<CakeKind> {
            match n {
                0 => Some(CakeKind::Basic),
                1 => Some(CakeKind::Spicy),
                2 => Some(CakeKind::Fruity),
                _ => Option::None
            }
        }
        pub fn to_raw(&self) -> u32 {
            *self as u32
        }
    }

    bitflags! { #[doc = r#"possible delivery modes

"#]
    pub struct DeliveryKind: u32 {
        const PickUp = 1;
        const Drone = 2;
        const Catapult = 4;
    } }
    impl DeliveryKind {
        pub fn from_raw(n: u32) -> Option<DeliveryKind> {
            Some(DeliveryKind::from_bits_truncate(n))
        }
        pub fn to_raw(&self) -> u32 {
            self.bits()
        }
    }

    pub struct Implementation<ID> {
        /// do some foo
        ///
        /// This will do some foo with its args.
        ///
        /// **Arguments:** event_queue_handle, interface_data, wl_foo, number, unumber, text, float, file
        pub foo_it: fn(evqh: &mut EventLoopHandle, data: &mut ID, client: &Client,  wl_foo: &WlFoo, number: i32, unumber: u32, text: String, float: f64, file: ::std::os::unix::io::RawFd),
        /// create a bar
        ///
        /// Create a bar which will do its bar job.
        ///
        /// **Arguments:** event_queue_handle, interface_data, wl_foo, id
        pub create_bar: fn(evqh: &mut EventLoopHandle, data: &mut ID, client: &Client,  wl_foo: &WlFoo, id: super::wl_bar::WlBar),
    }
    impl<ID> Copy for Implementation<ID> {}
    impl<ID> Clone for Implementation<ID> {
        fn clone(&self) -> Implementation<ID> {
            *self
        }
    }

    impl<ID> PartialEq for Implementation<ID> {
        fn eq(&self, other: &Implementation<ID>) -> bool {
            true
            && (self.foo_it as usize == other.foo_it as usize)
            && (self.create_bar as usize == other.create_bar as usize)

        }
    }

    const WL_FOO_CAKE: u32 = 0;

    impl WlFoo {
        /// a cake is possible
        ///
        /// The server advertizes that a kind of cake is available
        ///
        /// This event is only available since version 2 of the interface
        pub fn cake(&self, kind: CakeKind, amount: u32) ->EventResult<()> {
            if self.status() == Liveness::Dead { return EventResult::Destroyed }
            unsafe { ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_resource_post_event, self.ptr(), WL_FOO_CAKE, kind, amount) };
            EventResult::Sent(())
        }

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

    use super::{Liveness, Implementable};
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

    unsafe impl Resource for WlBar {
        fn ptr(&self) -> *mut wl_resource { self.ptr }

        unsafe fn from_ptr_new(ptr: *mut wl_resource) -> WlBar {
            let data = Box::into_raw(Box::new((
                ptr::null_mut::<c_void>(),
                Option::None::<Box<Any>>,
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
                let data = ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_resource_get_user_data, ptr) as *mut (*mut c_void, Option<Box<Any>>, Arc<(AtomicBool, AtomicPtr<()>)>);
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
        unsafe fn clone_unchecked(&self) -> WlBar {
            WlBar {
                ptr: self.ptr,
                data: self.data.clone()
            }
        }
    }

    unsafe impl<ID: 'static> Implementable<ID> for WlBar {
        type Implementation = Implementation<ID>;
        #[allow(unused_mut,unused_assignments)]
        unsafe fn __dispatch_msg(&self, client: &Client, opcode: u32, args: *const wl_argument) -> Result<(),()> {

        let data: &mut (*mut EventLoopHandle, Option<Box<Any>>, Arc<(AtomicBool, AtomicPtr<()>)>) =
            &mut *(ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_resource_get_user_data, self.ptr()) as *mut _);
        let evq = &mut *(data.0);
        let mut kill = false;
        {
            let &mut (ref implementation, ref mut idata) = data.1.as_mut().unwrap().downcast_mut::<(Implementation<ID>, ID)>().unwrap();
            match opcode {
                0 => {
                    let kind = {match super::wl_foo::DeliveryKind::from_raw(*(args.offset(0) as *const u32)) { Some(v) => v, Option::None => return Err(()) }};
                    let target = {Resource::from_ptr_initialized(*(args.offset(1) as *const *mut wl_resource))};
                    let metadata = {let array = *(args.offset(2) as *const *mut wl_array); ::std::slice::from_raw_parts((*array).data as *const u8, (*array).size as usize).to_owned()};
                    (implementation.bar_delivery)(evq, idata, client, self, kind, &target, metadata);
                },
                1 => {

                (data.2).0.store(false, ::std::sync::atomic::Ordering::SeqCst);
                kill = true;
                ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_resource_destroy, self.ptr());
                    (implementation.release)(evq, idata, client, self);
                },
                _ => return Err(())
            }
        }

        if kill {
            let _impl = data.1.take();
            ::std::mem::drop(_impl);
        }
            Ok(())
        }
    }

    pub struct Implementation<ID> {
        /// ask for a bar delivery
        ///
        /// Proceed to a bar delivery of given foo.
        ///
        /// **Arguments:** event_queue_handle, interface_data, wl_bar, kind, target, metadata
        ///
        /// This event only exists since version 2 of the interface
        pub bar_delivery: fn(evqh: &mut EventLoopHandle, data: &mut ID, client: &Client,  wl_bar: &WlBar, kind: super::wl_foo::DeliveryKind, target: &super::wl_foo::WlFoo, metadata: Vec<u8>),
        /// release this bar
        ///
        /// Notify the compositor that you have finished using this bar.
        ///
        /// **Arguments:** event_queue_handle, interface_data, wl_bar
        ///
        /// This is a destructor, you cannot send events to this object once this method is called.
        pub release: fn(evqh: &mut EventLoopHandle, data: &mut ID, client: &Client,  wl_bar: &WlBar),
    }

    impl<ID> Copy for Implementation<ID> {}
    impl<ID> Clone for Implementation<ID> {
        fn clone(&self) -> Implementation<ID> {
            *self
        }
    }

    impl<ID> PartialEq for Implementation<ID> {
        fn eq(&self, other: &Implementation<ID>) -> bool {
            true
            && (self.bar_delivery as usize == other.bar_delivery as usize)
            && (self.release as usize == other.release as usize)

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
    use super::{Liveness, Implementable};
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
    unsafe impl Resource for WlCallback {
        fn ptr(&self) -> *mut wl_resource { self.ptr }

        unsafe fn from_ptr_new(ptr: *mut wl_resource) -> WlCallback {
            let data = Box::into_raw(Box::new((
                ptr::null_mut::<c_void>(),
                Option::None::<Box<Any>>,
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
                let data = ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_resource_get_user_data, ptr) as *mut (*mut c_void, Option<Box<Any>>, Arc<(AtomicBool, AtomicPtr<()>)>);
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
        unsafe fn clone_unchecked(&self) -> WlCallback {
            WlCallback {
                ptr: self.ptr,
                data: self.data.clone()
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

