use std::os::raw::c_void;

use wayland_commons::{Implementation, Interface};

use NewResource;

#[cfg(feature = "native_lib")]
use wayland_sys::server::*;

/// A handle to a global object
///
/// This is given to you when you register a global to the event loop.
///
/// This handle allows you do destroy the global when needed.
///
/// If you know you will never destroy this global, you can let this
/// handle go out of scope.
pub struct Global<I: Interface> {
    _i: ::std::marker::PhantomData<*const I>,
    #[cfg(feature = "native_lib")]
    ptr: *mut wl_global,
    #[cfg(feature = "native_lib")]
    data: *mut Box<Implementation<NewResource<I>, u32>>,
}

impl<I: Interface> Global<I> {
    #[cfg(feature = "native_lib")]
    pub(crate) unsafe fn create(
        ptr: *mut wl_global,
        data: Box<Box<Implementation<NewResource<I>, u32>>>,
    ) -> Global<I> {
        Global {
            _i: ::std::marker::PhantomData,
            ptr: ptr,
            data: Box::into_raw(data),
        }
    }

    /// Destroy the associated global object.
    pub fn destroy(self) {
        #[cfg(not(feature = "native_lib"))]
        {}
        #[cfg(feature = "native_lib")]
        unsafe {
            // destroy the global
            ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_global_destroy, self.ptr);
            // free the user data
            let data = Box::from_raw(self.data);
            drop(data);
        }
    }
}

#[cfg(feature = "native_lib")]
pub(crate) unsafe extern "C" fn global_bind<I: Interface>(
    client: *mut wl_client,
    data: *mut c_void,
    version: u32,
    id: u32,
) {
    // safety of this function is the same as dispatch_func
    let ret = ::std::panic::catch_unwind(move || {
        let implem = &mut *(data as *mut Box<Implementation<NewResource<I>, u32>>);
        let ptr = ffi_dispatch!(
            WAYLAND_SERVER_HANDLE,
            wl_resource_create,
            client,
            I::c_interface(),
            version as i32, // wayland already checks the validity of the version
            id
        );
        let resource = NewResource::from_c_ptr(ptr as *mut wl_resource);
        implem.receive(version, resource);
    });
    match ret {
        Ok(()) => (), // all went well
        Err(_) => {
            // a panic occured
            eprintln!(
                "[wayland-server error] A global handler for {} panicked, aborting.",
                I::NAME
            );
            ::libc::abort();
        }
    }
}
