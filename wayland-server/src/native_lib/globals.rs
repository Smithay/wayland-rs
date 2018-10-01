use std::cell::RefCell;
use std::os::raw::c_void;
use std::rc::Rc;

use wayland_commons::Interface;

use wayland_sys::server::*;

use super::ClientInner;
use NewResource;

pub(crate) struct GlobalData<I: Interface> {
    pub(crate) bind: Box<FnMut(NewResource<I>, u32)>,
    pub(crate) filter: Option<Box<FnMut(ClientInner) -> bool>>,
}

impl<I: Interface> GlobalData<I> {
    pub(crate) fn new<F1, F2>(bind: F1, filter: Option<F2>) -> GlobalData<I>
    where
        F1: FnMut(NewResource<I>, u32) + 'static,
        F2: FnMut(ClientInner) -> bool + 'static,
    {
        GlobalData {
            bind: Box::new(bind) as Box<_>,
            filter: filter.map(|f| Box::new(f) as Box<_>),
        }
    }
}

pub(crate) struct GlobalInner<I: Interface> {
    ptr: *mut wl_global,
    data: *mut GlobalData<I>,
    rust_globals: Rc<RefCell<Vec<*mut wl_global>>>,
}

impl<I: Interface> GlobalInner<I> {
    pub(crate) unsafe fn create(
        ptr: *mut wl_global,
        data: Box<GlobalData<I>>,
        rust_globals: Rc<RefCell<Vec<*mut wl_global>>>,
    ) -> GlobalInner<I> {
        GlobalInner {
            ptr: ptr,
            data: Box::into_raw(data),
            rust_globals,
        }
    }

    pub fn destroy(self) {
        unsafe {
            // destroy the global
            ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_global_destroy, self.ptr);
            // remove from the list
            self.rust_globals.borrow_mut().retain(|&g| g != self.ptr);
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
        let data = &mut *(data as *mut GlobalData<I>);
        let ptr = ffi_dispatch!(
            WAYLAND_SERVER_HANDLE,
            wl_resource_create,
            client,
            I::c_interface(),
            version as i32, // wayland already checks the validity of the version
            id
        );
        let resource = NewResource::from_c_ptr(ptr as *mut wl_resource);
        (data.bind)(resource, version);
    });
    match ret {
        Ok(()) => (), // all went well
        Err(_) => {
            // a panic occurred
            eprintln!(
                "[wayland-server error] A global handler for {} panicked, aborting.",
                I::NAME
            );
            ::libc::abort();
        }
    }
}

pub(crate) unsafe extern "C" fn global_filter(
    client: *const wl_client,
    global: *const wl_global,
    data: *mut c_void,
) -> bool {
    // safety of this function is the same as dispatch_func
    let ret = ::std::panic::catch_unwind(move || {
        // early exit with true if the global is not rust-managed
        let rust_globals = &*(data as *const RefCell<Vec<*mut wl_global>>);
        if rust_globals
            .borrow()
            .iter()
            .all(|&g| g as *const wl_global != global)
        {
            return true;
        }
        // the global is rust-managed, continue
        let global_data = ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_global_get_user_data, global)
            as *mut GlobalData<::AnonymousObject>;
        let client = ClientInner::from_ptr(client as *mut _);
        let filter = &mut (*global_data).filter;
        if let Some(ref mut filter) = *filter {
            filter(client)
        } else {
            true
        }
    });
    match ret {
        Ok(val) => val, // all went well
        Err(_) => {
            // a panic occurred
            eprintln!("[wayland-server error] A global filter panicked, aborting.");
            ::libc::abort();
        }
    }
}
