#[cfg(feature = "native_lib")]
use std::ffi::{CString, OsString};
use std::os::raw::c_void;
#[cfg(feature = "native_lib")]
use std::os::unix::ffi::OsStringExt;
use std::sync::Arc;

use wayland_commons::Interface;

use {Client, EventLoop, Global, GlobalImplementation, LoopToken};
use globals::global_bind;

#[cfg(feature = "native_lib")]
use wayland_sys::server::*;

pub(crate) struct DisplayInner {
    #[cfg(feature = "native_lib")] pub(crate) ptr: *mut wl_display,
}

impl Drop for DisplayInner {
    fn drop(&mut self) {
        #[cfg(not(feature = "native_lib"))]
        {
            unimplemented!();
        }
        #[cfg(feature = "native_lib")]
        {
            unsafe {
                ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_display_destroy, self.ptr);
            }
        }
    }
}

pub struct Display {
    inner: Arc<DisplayInner>,
}

impl Display {
    #[cfg(feature = "native_lib")]
    pub fn new() -> (Display, EventLoop) {
        let ptr = unsafe { ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_display_create,) };

        let display = Display {
            inner: Arc::new(DisplayInner { ptr: ptr }),
        };

        // setup the client_created listener
        unsafe {
            let listener = signal::rust_listener_create(client_created);
            ffi_dispatch!(
                WAYLAND_SERVER_HANDLE,
                wl_display_add_client_created_listener,
                ptr,
                listener
            );
        }

        let evq_ptr = unsafe { ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_display_get_event_loop, ptr) };

        let evq = unsafe { EventLoop::display_new(display.inner.clone(), evq_ptr) };

        (display, evq)
    }

    pub fn create_global<I: Interface, ID: 'static>(
        &mut self,
        token: &LoopToken,
        version: u32,
        implementation: GlobalImplementation<I, ID>,
        idata: ID,
    ) -> Global<I, ID> {
        let token_inner = token
            .inner
            .inner
            .as_ref()
            .expect("Display::create_global requires the token associated with the display event loop.");
        assert!(
            Arc::ptr_eq(&self.inner, token_inner),
            "Display::create_global requires the token associated with the display event loop."
        );

        let data = Box::new((implementation, idata));

        unsafe {
            let ptr = ffi_dispatch!(
                WAYLAND_SERVER_HANDLE,
                wl_global_create,
                self.inner.ptr,
                I::c_interface(),
                version as i32,
                &*data as *const (GlobalImplementation<I, ID>, ID) as *mut _,
                global_bind::<I, ID>
            );

            Global::create(ptr, data)
        }
    }
}

#[cfg(feature = "native_lib")]
unsafe extern "C" fn client_created(listener: *mut wl_listener, data: *mut c_void) {
    // init the client
    let _client = Client::from_ptr(data as *mut wl_client);
}
