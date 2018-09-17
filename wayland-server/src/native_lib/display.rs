use std::cell::RefCell;
use std::ffi::{CStr, CString, OsStr, OsString};
use std::io::{Error as IoError, ErrorKind, Result as IoResult};
use std::os::raw::c_void;
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::os::unix::io::RawFd;
use std::ptr;
use std::rc::Rc;

use wayland_sys::server::*;

use calloop::generic::Generic;
use calloop::{LoopHandle, Source};

use Fd;

use super::globals::GlobalData;
use super::{ClientInner, GlobalInner};

use display::get_runtime_dir;
use {Interface, NewResource};

pub(crate) struct DisplayInner {
    pub(crate) ptr: *mut wl_display,
    source: Option<Source<Generic<Fd>>>,
}

impl Drop for DisplayInner {
    fn drop(&mut self) {
        {
            self.source.take().map(|s| s.remove());
            unsafe {
                ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_display_destroy, self.ptr);
            }
        }
    }
}

impl DisplayInner {
    pub(crate) fn new<Data: 'static>(handle: LoopHandle<Data>) -> Rc<RefCell<DisplayInner>> {
        unsafe {
            let ptr = ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_display_create,);
            // setup the client_created listener
            let listener = signal::rust_listener_create(client_created);
            ffi_dispatch!(
                WAYLAND_SERVER_HANDLE,
                wl_display_add_client_created_listener,
                ptr,
                listener
            );
            // setup the global filter
            ffi_dispatch!(
                WAYLAND_SERVER_HANDLE,
                wl_display_set_global_filter,
                ptr,
                super::globals::global_filter,
                ::std::ptr::null_mut()
            );

            let evl_ptr = ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_display_get_event_loop, ptr);
            let evl_fd = ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_event_loop_get_fd, evl_ptr);

            let mut evtsrc = Generic::new(Fd(evl_fd));
            evtsrc.set_interest(::mio::Ready::readable());
            evtsrc.set_pollopts(::mio::PollOpt::edge());

            let source = Some(
                handle
                    .insert_source(evtsrc, move |_, _| {
                        ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_event_loop_dispatch, evl_ptr, 0);
                    }).unwrap(),
            );

            Rc::new(RefCell::new(DisplayInner { ptr, source }))
        }
    }

    pub(crate) fn ptr(&self) -> *mut wl_display {
        self.ptr
    }

    pub(crate) fn create_global<I: Interface, F1, F2>(
        &mut self,
        version: u32,
        implementation: F1,
        filter: Option<F2>,
    ) -> GlobalInner<I>
    where
        F1: FnMut(NewResource<I>, u32) + 'static,
        F2: FnMut(ClientInner) -> bool + 'static,
    {
        let data = Box::new(GlobalData::new(implementation, filter));

        unsafe {
            let ptr = ffi_dispatch!(
                WAYLAND_SERVER_HANDLE,
                wl_global_create,
                self.ptr,
                I::c_interface(),
                version as i32,
                &*data as *const GlobalData<I> as *mut _,
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
