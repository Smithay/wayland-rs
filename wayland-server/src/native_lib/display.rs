use std::cell::RefCell;
use std::ffi::{CStr, CString, OsStr, OsString};
use std::io::{Error as IoError, ErrorKind, Result as IoResult};
use std::os::raw::c_void;
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::os::unix::io::RawFd;
use std::ptr;
use std::rc::Rc;

use wayland_sys::server::*;

use super::{ClientInner, EventLoopInner, GlobalInner};

use display::get_runtime_dir;
use {Implementation, Interface, NewResource};

pub(crate) struct DisplayInner {
    pub(crate) ptr: *mut wl_display,
}

impl Drop for DisplayInner {
    fn drop(&mut self) {
        {
            unsafe {
                ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_display_destroy, self.ptr);
            }
        }
    }
}

impl DisplayInner {
    pub(crate) fn new() -> (Rc<RefCell<DisplayInner>>, EventLoopInner) {
        unsafe {
            let ptr = ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_display_create,);
            let display = Rc::new(RefCell::new(DisplayInner { ptr: ptr }));
            // setup the client_created listener
            let listener = signal::rust_listener_create(client_created);
            ffi_dispatch!(
                WAYLAND_SERVER_HANDLE,
                wl_display_add_client_created_listener,
                ptr,
                listener
            );
            let evq_ptr = ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_display_get_event_loop, ptr);
            let evq = EventLoopInner::display_new(display.clone(), evq_ptr);
            (display, evq)
        }
    }

    pub(crate) fn ptr(&self) -> *mut wl_display {
        self.ptr
    }

    pub(crate) fn create_global<I: Interface, Impl>(
        &mut self,
        _: &EventLoopInner,
        version: u32,
        implementation: Impl,
    ) -> GlobalInner<I>
    where
        Impl: Implementation<NewResource<I>, u32> + 'static,
    {
        let data = Box::new(Box::new(implementation) as Box<Implementation<NewResource<I>, u32>>);

        unsafe {
            let ptr = ffi_dispatch!(
                WAYLAND_SERVER_HANDLE,
                wl_global_create,
                self.ptr,
                I::c_interface(),
                version as i32,
                &*data as *const Box<_> as *mut _,
                super::globals::global_bind::<I>
            );

            GlobalInner::create(ptr, data)
        }
    }

    pub(crate) fn flush_clients(&mut self) {
        unsafe { ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_display_flush_clients, self.ptr) };
    }

    pub(crate) fn add_socket<S>(&mut self, name: Option<S>) -> IoResult<()>
    where
        S: AsRef<OsStr>,
    {
        let cname = match name.as_ref().map(|s| CString::new(s.as_ref().as_bytes())) {
            Some(Ok(n)) => Some(n),
            Some(Err(_)) => {
                return Err(IoError::new(
                    ErrorKind::InvalidInput,
                    "nulls are not allowed in socket name",
                ))
            }
            None => None,
        };
        let ret = unsafe {
            ffi_dispatch!(
                WAYLAND_SERVER_HANDLE,
                wl_display_add_socket,
                self.ptr,
                cname.as_ref().map(|s| s.as_ptr()).unwrap_or(ptr::null())
            )
        };
        if ret == -1 {
            // lets try to be helpfull
            let mut socket_name = get_runtime_dir()?;
            match name {
                Some(s) => socket_name.push(s.as_ref()),
                None => socket_name.push("wayland-0"),
            }
            Err(IoError::new(
                ErrorKind::PermissionDenied,
                format!("could not bind socket {}", socket_name.to_string_lossy()),
            ))
        } else {
            Ok(())
        }
    }

    pub(crate) fn add_socket_auto(&mut self) -> IoResult<OsString> {
        let ret = unsafe { ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_display_add_socket_auto, self.ptr) };
        if ret.is_null() {
            // try to be helpfull
            let socket_name = get_runtime_dir()?;
            Err(IoError::new(
                ErrorKind::Other,
                format!("no available wayland-* name in {}", socket_name.to_string_lossy()),
            ))
        } else {
            let sockname = unsafe { CStr::from_ptr(ret) };
            Ok(<OsString as OsStringExt>::from_vec(sockname.to_bytes().into()))
        }
    }

    pub(crate) unsafe fn add_socket_fd(&mut self, fd: RawFd) -> IoResult<()> {
        let ret = ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_display_add_socket_fd, self.ptr, fd);
        if ret == 0 {
            Ok(())
        } else {
            Err(IoError::new(ErrorKind::InvalidInput, "invalid socket fd"))
        }
    }

    pub unsafe fn create_client(&mut self, fd: RawFd) -> ClientInner {
        let ret = ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_client_create, self.ptr, fd);
        ClientInner::from_ptr(ret)
    }
}

unsafe extern "C" fn client_created(_listener: *mut wl_listener, data: *mut c_void) {
    // init the client
    let _client = ClientInner::from_ptr(data as *mut wl_client);
}
