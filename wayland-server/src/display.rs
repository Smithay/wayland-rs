

use event_loop::{create_event_loop, EventLoop};
use std::env;
use std::ffi::{CStr, CString, OsStr, OsString};
use std::io::{Error as IoError, ErrorKind, Result as IoResult};
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::os::unix::io::{IntoRawFd, RawFd};
use std::path::PathBuf;
use std::ptr;
use wayland_sys::server::*;

/// A wayland socket
///
/// This represents a socket your compositor can receive clients on.
pub struct Display {
    ptr: *mut wl_display,
}

/// Create a new display
///
/// This display does not listen on any socket by default. You'll need to add one (or more)
/// using the `add_socket_*` methods.
pub fn create_display() -> (Display, EventLoop) {
    unsafe {
        let ptr = ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_display_create,);
        let el_ptr = ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_display_get_event_loop, ptr);
        (Display { ptr: ptr }, create_event_loop(el_ptr, Some(ptr)))
    }
}

impl Display {
    /// Add a listening socket to this display
    ///
    /// Wayland clients will be able to connect to your compositor from this socket.
    ///
    /// Socket will be created in the directory specified by the environment variable
    /// `XDG_RUNTIME_DIR`.
    ///
    /// If a name is provided, it is used. Otherwise, if `WAYLAND_DISPLAY` environment
    /// variable is set, its contents are used as socket name. Otherwise, `wayland-0` is used.
    ///
    /// Errors if `name` contains an interior null, or if `XDG_RUNTIME_DIR` is not set,
    /// or if specified could not be bound (either it is already used or the compositor
    /// does not have the rights to create it).
    pub fn add_socket<S>(&mut self, name: Option<S>) -> IoResult<()>
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

    /// Add an automatically named listening socket to this display
    ///
    /// Wayland clients will be able to connect to your compositor from this socket.
    ///
    /// Socket will be created in the directory specified by the environment variable
    /// `XDG_RUNTIME_DIR`. The directory is scanned for any name in the form `wayland-$d` with
    /// `0 <= $d < 32` and the first available one is used.
    ///
    /// Errors if `XDG_RUNTIME_DIR` is not set, or all 32 names are already in use.
    pub fn add_socket_auto(&mut self) -> IoResult<OsString> {
        let ret = unsafe { ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_display_add_socket_auto, self.ptr) };
        if ret.is_null() {
            // try to be helpfull
            let socket_name = get_runtime_dir()?;
            Err(IoError::new(
                ErrorKind::Other,
                format!(
                    "no available wayland-* name in {}",
                    socket_name.to_string_lossy()
                ),
            ))
        } else {
            let sockname = unsafe { CStr::from_ptr(ret) };
            Ok(<OsString as OsStringExt>::from_vec(
                sockname.to_bytes().into(),
            ))
        }
    }

    /// Add existing listening socket to this display
    ///
    /// Wayland clients will be able to connect to your compositor from this socket.
    ///
    /// The existing socket fd must already be created, opened, and locked.
    /// The fd must be properly set to CLOEXEC and bound to a socket file
    /// with both bind() and listen() already called. An error is returned
    /// otherwise.
    pub fn add_socket_from<T>(&mut self, socket: T) -> IoResult<()>
    where
        T: IntoRawFd,
    {
        unsafe { self.add_socket_fd(socket.into_raw_fd()) }
    }

    /// Add existing listening socket to this display from a raw file descriptor
    ///
    /// Wayland clients will be able to connect to your compositor from this socket.
    ///
    /// The library takes ownership of the provided socket if this method returns
    /// successfully.
    ///
    /// The existing socket fd must already be created, opened, and locked.
    /// The fd must be properly set to CLOEXEC and bound to a socket file
    /// with both bind() and listen() already called. An error is returned
    /// otherwise.
    pub unsafe fn add_socket_fd(&mut self, fd: RawFd) -> IoResult<()> {
        let ret = ffi_dispatch!(
            WAYLAND_SERVER_HANDLE,
            wl_display_add_socket_fd,
            self.ptr,
            fd
        );
        if ret == 0 {
            Ok(())
        } else {
            Err(IoError::new(ErrorKind::InvalidInput, "invalid socket fd"))
        }
    }

    /// Flush events to the clients
    ///
    /// Will send as many pending events as possible to the respective sockets of the clients.
    /// Will not block, but might not send everything if the socket buffer fills up.
    pub fn flush_clients(&self) {
        unsafe { ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_display_flush_clients, self.ptr) };
    }

    /// Obtain an FFI pointer
    ///
    /// This provides an FFI pointer to the underlying `*mut wl_display`. You'll typically
    /// need it to interface with MESA for example.
    ///
    /// **Unsafety:** This pointer becomes invalid once the `Display` is dropped.
    pub unsafe fn ptr(&self) -> *mut wl_display {
        self.ptr
    }
}

impl Drop for Display {
    fn drop(&mut self) {
        unsafe {
            ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_display_destroy, self.ptr);
        }
    }
}

fn get_runtime_dir() -> IoResult<PathBuf> {
    match env::var_os("XDG_RUNTIME_DIR") {
        Some(s) => Ok(s.into()),
        None => Err(IoError::new(
            ErrorKind::NotFound,
            "XDG_RUNTIME_DIR env variable is not set",
        )),
    }
}
