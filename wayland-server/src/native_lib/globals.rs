use std::os::raw::c_void;

use wayland_commons::{Implementation, Interface};

use wayland_sys::server::*;

use NewResource;

pub(crate) struct GlobalInner<I: Interface> {
    _i: ::std::marker::PhantomData<*const I>,
    ptr: *mut wl_global,
    data: *mut Box<Implementation<NewResource<I>, u32>>,
}

impl<I: Interface> GlobalInner<I> {
    pub(crate) unsafe fn create(
        ptr: *mut wl_global,
        data: Box<Box<Implementation<NewResource<I>, u32>>>,
    ) -> GlobalInner<I> {
        GlobalInner {
            _i: ::std::marker::PhantomData,
            ptr: ptr,
            data: Box::into_raw(data),
        }
    }

    pub fn destroy(self) {
        unsafe {
            // destroy the global
            ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_global_destroy, self.ptr);
            // free the user data
            let data = Box::from_raw(self.data);
            drop(data);
        }
    }
}

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
