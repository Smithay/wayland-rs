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
    use super::EventQueueHandle;
    use super::Proxy;
    use super::RequestResult;
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
    use wayland_sys::client::*;
    type UserData = (*mut EventQueueHandle, Option<Box<Any>>, Arc<(AtomicBool, AtomicPtr<()>)>);

    pub struct WlFoo {
        ptr: *mut wl_proxy,
        data: Option<Arc<(AtomicBool, AtomicPtr<()>)>>
    }

    unsafe impl Send for WlFoo {}
    unsafe impl Sync for WlFoo {}

    unsafe impl Proxy for WlFoo {
        fn ptr(&self) -> *mut wl_proxy { self.ptr }

        unsafe fn from_ptr_new(ptr: *mut wl_proxy) -> WlFoo {
            let data: *mut UserData = Box::into_raw(Box::new((
                ptr::null_mut(),
                Option::None,
                Arc::new((AtomicBool::new(true), AtomicPtr::new(ptr::null_mut()))),
            )));
            ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_set_user_data, ptr, data as *mut c_void);
            WlFoo { ptr: ptr, data: Some((&*data).2.clone()) }
        }
        unsafe fn from_ptr_initialized(ptr: *mut wl_proxy) -> WlFoo {

            let implem = ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_get_listener, ptr);
            let rust_managed = implem == &RUST_MANAGED as *const _ as *const _;


            if rust_managed {
                let data = ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_get_user_data, ptr) as *mut UserData;
                WlFoo { ptr: ptr, data: Some((&*data).2.clone()) }
            } else {
                WlFoo { ptr: ptr, data: Option::None }
            }
        }

        fn interface_ptr() -> *const wl_interface { unsafe { &wl_foo_interface } }
        fn interface_name() -> &'static str { "wl_foo"  }
        fn supported_version() -> u32 { 3 }
        fn version(&self) -> u32 { unsafe { ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_get_version, self.ptr()) } }

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
        unsafe fn __dispatch_msg(&self,  opcode: u32, args: *const wl_argument) -> Result<(),()> {

        let data = &mut *(ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_get_user_data, self.ptr()) as *mut UserData);
        let evq = &mut *(data.0);
        let mut kill = false;
        {
            let &mut (ref implementation, ref mut idata) = data.1.as_mut().unwrap().downcast_mut::<(Implementation<ID>, ID)>().unwrap();
            match opcode {
                0 => {
                    let kind = {match CakeKind::from_raw(*(args.offset(0) as *const u32)) { Some(v) => v, Option::None => return Err(()) }};
                    let amount = {*(args.offset(1) as *const u32)};
                    (implementation.cake)(evq, idata,  self, kind, amount);
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
        /// a cake is possible
        ///
        /// The server advertizes that a kind of cake is available
        ///
        /// **Arguments:** event_queue_handle, interface_data, wl_foo, kind, amount
        ///
        /// This request only exists since version 2 of the interface
        pub cake: fn(evqh: &mut EventQueueHandle, data: &mut ID,  wl_foo: &WlFoo, kind: CakeKind, amount: u32),
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
            && (self.cake as usize == other.cake as usize)

        }
    }


    const WL_FOO_FOO_IT: u32 = 0;
    const WL_FOO_CREATE_BAR: u32 = 1;
    impl WlFoo {
        /// do some foo
        ///
        /// This will do some foo with its args.
        pub fn foo_it(&self, number: i32, unumber: u32, text: String, float: f64, file: ::std::os::unix::io::RawFd) ->() {
            let text = CString::new(text).unwrap_or_else(|_| panic!("Got a String with interior null in wl_foo.foo_it:text"));
            let float = wl_fixed_from_double(float);
            unsafe { ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_marshal, self.ptr(), WL_FOO_FOO_IT, number, unumber, text.as_ptr(), float, file) };
        }

        /// create a bar
        ///
        /// Create a bar which will do its bar job.
        pub fn create_bar(&self) ->super::wl_bar::WlBar {
            let ptr = unsafe { ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_marshal_constructor, self.ptr(), WL_FOO_CREATE_BAR, &wl_bar_interface, ptr::null_mut::<wl_proxy>()) };
            let proxy = unsafe { Proxy::from_ptr_new(ptr) };
            proxy
        }
    }
}
pub mod wl_bar {
    //! Interface for bars
    //!
    //! This interface allows you to bar your foos.
    use super::EventQueueHandle;
    use super::Proxy;
    use super::RequestResult;

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
    use wayland_sys::client::*;
    type UserData = (*mut EventQueueHandle, Option<Box<Any>>, Arc<(AtomicBool, AtomicPtr<()>)>);

    pub struct WlBar {
        ptr: *mut wl_proxy,
        data: Option<Arc<(AtomicBool, AtomicPtr<()>)>>
    }

    unsafe impl Send for WlBar {}
    unsafe impl Sync for WlBar {}

    unsafe impl Proxy for WlBar {
        fn ptr(&self) -> *mut wl_proxy { self.ptr }

        unsafe fn from_ptr_new(ptr: *mut wl_proxy) -> WlBar {
            let data: *mut UserData = Box::into_raw(Box::new((
                ptr::null_mut(),
                Option::None,
                Arc::new((AtomicBool::new(true), AtomicPtr::new(ptr::null_mut()))),
            )));
            ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_set_user_data, ptr, data as *mut c_void);
            WlBar { ptr: ptr, data: Some((&*data).2.clone()) }
        }
        unsafe fn from_ptr_initialized(ptr: *mut wl_proxy) -> WlBar {

            let implem = ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_get_listener, ptr);
            let rust_managed = implem == &RUST_MANAGED as *const _ as *const _;


            if rust_managed {
                let data = ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_get_user_data, ptr) as *mut UserData;
                WlBar { ptr: ptr, data: Some((&*data).2.clone()) }
            } else {
                WlBar { ptr: ptr, data: Option::None }
            }
        }

        fn interface_ptr() -> *const wl_interface { unsafe { &wl_bar_interface } }
        fn interface_name() -> &'static str { "wl_bar"  }
        fn supported_version() -> u32 { 1 }
        fn version(&self) -> u32 { unsafe { ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_get_version, self.ptr()) } }

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

    const WL_BAR_BAR_DELIVERY: u32 = 0;
    const WL_BAR_RELEASE: u32 = 1;

    impl WlBar {
        /// ask for a bar delivery
        ///
        /// Proceed to a bar delivery of given foo.
        ///
        /// This request is only available since version 2 of the interface
        pub fn bar_delivery(&self, kind: super::wl_foo::DeliveryKind, target: &super::wl_foo::WlFoo, metadata: Vec<u8>) ->RequestResult<()> {
            if self.status() == Liveness::Dead { return RequestResult::Destroyed }
            let metadata = wl_array { size: metadata.len(), alloc: metadata.capacity(), data: metadata.as_ptr() as *mut _ };
            unsafe { ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_marshal, self.ptr(), WL_BAR_BAR_DELIVERY, kind, target.ptr(), &metadata as *const wl_array) };
            RequestResult::Sent(())
        }

        /// release this bar
        ///
        /// Notify the compositor that you have finished using this bar.
        ///
        /// This is a destructor, you cannot send requests to this object once this method is called.
        pub fn release(&self) ->RequestResult<()> {
            if self.status() == Liveness::Dead { return RequestResult::Destroyed }
            unsafe { ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_marshal, self.ptr(), WL_BAR_RELEASE) };
            if let Some(ref data) = self.data {
                data.0.store(false, ::std::sync::atomic::Ordering::SeqCst);
            }
            unsafe { ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_destroy, self.ptr()); }
            RequestResult::Sent(())
        }
    }
}

pub mod wl_display {
    //! core global object
    //!
    //! This global is special and should only generate code client-side, not server-side.
    use super::EventQueueHandle;
    use super::Proxy;
    use super::RequestResult;
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
    use wayland_sys::client::*;
    type UserData = (*mut EventQueueHandle, Option<Box<Any>>, Arc<(AtomicBool, AtomicPtr<()>)>);

    pub struct WlDisplay {
        ptr: *mut wl_proxy,
        data: Option<Arc<(AtomicBool, AtomicPtr<()>)>>
    }

    unsafe impl Send for WlDisplay {}
    unsafe impl Sync for WlDisplay {}
    unsafe impl Proxy for WlDisplay {
        fn ptr(&self) -> *mut wl_proxy { self.ptr }

        unsafe fn from_ptr_new(ptr: *mut wl_proxy) -> WlDisplay {
            let data: *mut UserData = Box::into_raw(Box::new((
                ptr::null_mut(),
                Option::None,
                Arc::new((AtomicBool::new(true), AtomicPtr::new(ptr::null_mut()))),
            )));
            ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_set_user_data, ptr, data as *mut c_void);
            WlDisplay { ptr: ptr, data: Some((&*data).2.clone()) }
        }
        unsafe fn from_ptr_initialized(ptr: *mut wl_proxy) -> WlDisplay {

            let implem = ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_get_listener, ptr);
            let rust_managed = implem == &RUST_MANAGED as *const _ as *const _;

            if rust_managed {
                let data = ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_get_user_data, ptr) as *mut UserData;
                WlDisplay { ptr: ptr, data: Some((&*data).2.clone()) }
            } else {
                WlDisplay { ptr: ptr, data: Option::None }
            }
        }

        fn interface_ptr() -> *const wl_interface { unsafe { &wl_display_interface } }
        fn interface_name() -> &'static str { "wl_display"  }
        fn supported_version() -> u32 { 1 }
        fn version(&self) -> u32 { unsafe { ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_get_version, self.ptr()) } }

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

        fn equals(&self, other: &WlDisplay) -> bool {
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
        unsafe fn clone_unchecked(&self) -> WlDisplay {
            WlDisplay {
                ptr: self.ptr,
                data: self.data.clone()
            }
        }
    }
    impl WlDisplay {
    }
}
pub mod wl_registry {
    //! global registry object
    //!
    //! This global is special and should only generate code client-side, not server-side.
    use super::EventQueueHandle;
    use super::Proxy;
    use super::RequestResult;

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
    use wayland_sys::client::*;
    type UserData = (*mut EventQueueHandle, Option<Box<Any>>, Arc<(AtomicBool, AtomicPtr<()>)>);

    pub struct WlRegistry {
        ptr: *mut wl_proxy,
        data: Option<Arc<(AtomicBool, AtomicPtr<()>)>>
    }

    unsafe impl Send for WlRegistry {}
    unsafe impl Sync for WlRegistry {}
    unsafe impl Proxy for WlRegistry {
        fn ptr(&self) -> *mut wl_proxy { self.ptr }

        unsafe fn from_ptr_new(ptr: *mut wl_proxy) -> WlRegistry {
            let data: *mut UserData = Box::into_raw(Box::new((
                ptr::null_mut(),
                Option::None,
                Arc::new((AtomicBool::new(true), AtomicPtr::new(ptr::null_mut()))),
            )));
            ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_set_user_data, ptr, data as *mut c_void);
            WlRegistry { ptr: ptr, data: Some((&*data).2.clone()) }
        }
        unsafe fn from_ptr_initialized(ptr: *mut wl_proxy) -> WlRegistry {

            let implem = ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_get_listener, ptr);
            let rust_managed = implem == &RUST_MANAGED as *const _ as *const _;

            if rust_managed {
                let data = ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_get_user_data, ptr) as *mut UserData;
                WlRegistry { ptr: ptr, data: Some((&*data).2.clone()) }
            } else {
                WlRegistry { ptr: ptr, data: Option::None }
            }
        }

        fn interface_ptr() -> *const wl_interface { unsafe { &wl_registry_interface } }
        fn interface_name() -> &'static str { "wl_registry"  }
        fn supported_version() -> u32 { 1 }
        fn version(&self) -> u32 { unsafe { ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_get_version, self.ptr()) } }

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

        fn equals(&self, other: &WlRegistry) -> bool {
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
        unsafe fn clone_unchecked(&self) -> WlRegistry {
            WlRegistry {
                ptr: self.ptr,
                data: self.data.clone()
            }
        }
    }
    const WL_REGISTRY_BIND: u32 = 0;
    impl WlRegistry {
        /// bind an object to the display
        ///
        /// This request is a special code-path, as its new-id argument as no target type.
        pub fn bind<T: Proxy>(&self, version: u32, name: u32) ->T {
            if version > <T as Proxy>::supported_version() {
                panic!("Tried to bind interface {} with version {} while it is only supported up to {}.", <T as Proxy>::interface_name(), version, <T as Proxy>::supported_version())
            }
            let ptr = unsafe { ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_marshal_constructor_versioned, self.ptr(), WL_REGISTRY_BIND, <T as Proxy>::interface_ptr(), version, name, (*<T as Proxy>::interface_ptr()).name, version, ptr::null_mut::<wl_proxy>()) };
            let proxy = unsafe { Proxy::from_ptr_new(ptr) };
            proxy
        }
    }
}

pub mod wl_callback {
    //! callback object
    //!
    //! This object has a special behavior regarding its destructor.
    use super::EventQueueHandle;
    use super::Proxy;
    use super::RequestResult;

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
    use wayland_sys::client::*;
    type UserData = (*mut EventQueueHandle, Option<Box<Any>>, Arc<(AtomicBool, AtomicPtr<()>)>);

    pub struct WlCallback {
        ptr: *mut wl_proxy,
        data: Option<Arc<(AtomicBool, AtomicPtr<()>)>>
    }

    unsafe impl Send for WlCallback {}
    unsafe impl Sync for WlCallback {}
    unsafe impl Proxy for WlCallback {
        fn ptr(&self) -> *mut wl_proxy { self.ptr }

        unsafe fn from_ptr_new(ptr: *mut wl_proxy) -> WlCallback {
            let data: *mut UserData = Box::into_raw(Box::new((
                ptr::null_mut(),
                Option::None,
                Arc::new((AtomicBool::new(true), AtomicPtr::new(ptr::null_mut()))),

            )));
            ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_set_user_data, ptr, data as *mut c_void);
            WlCallback { ptr: ptr, data: Some((&*data).2.clone()) }
        }
        unsafe fn from_ptr_initialized(ptr: *mut wl_proxy) -> WlCallback {

            let implem = ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_get_listener, ptr);
            let rust_managed = implem == &RUST_MANAGED as *const _ as *const _;

            if rust_managed {
                let data = ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_get_user_data, ptr) as *mut UserData;
                WlCallback { ptr: ptr, data: Some((&*data).2.clone()) }
            } else {
                WlCallback { ptr: ptr, data: Option::None }
            }
        }

        fn interface_ptr() -> *const wl_interface { unsafe { &wl_callback_interface } }
        fn interface_name() -> &'static str { "wl_callback"  }
        fn supported_version() -> u32 { 1 }
        fn version(&self) -> u32 { unsafe { ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_get_version, self.ptr()) } }

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

    unsafe impl<ID: 'static> Implementable<ID> for WlCallback {
        type Implementation = Implementation<ID>;
        #[allow(unused_mut,unused_assignments)]
        unsafe fn __dispatch_msg(&self,  opcode: u32, args: *const wl_argument) -> Result<(),()> {
        let data = &mut *(ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_get_user_data, self.ptr()) as *mut UserData);
        let evq = &mut *(data.0);
        let mut kill = false;
        {
            let &mut (ref implementation, ref mut idata) = data.1.as_mut().unwrap().downcast_mut::<(Implementation<ID>, ID)>().unwrap();

            match opcode {
                0 => {
                    let callback_data = {*(args.offset(0) as *const u32)};

                (data.2).0.store(false, ::std::sync::atomic::Ordering::SeqCst);
                kill = true;
                ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_destroy, self.ptr());
                    (implementation.done)(evq, idata,  self, callback_data);
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
        /// done event
        ///
        /// This event is actually a destructor, but the protocol XML has no wait of specifying it.
        /// As such, the scanner should consider wl_callback.done as a special case.
        ///
        /// **Arguments:** event_queue_handle, interface_data, wl_callback, callback_data
        ///
        /// This is a destructor, you cannot send requests to this object once this method is called.
        pub done: fn(evqh: &mut EventQueueHandle, data: &mut ID,  wl_callback: &WlCallback, callback_data: u32),
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

            && (self.done as usize == other.done as usize)

        }
    }

    impl WlCallback {
    }
}

