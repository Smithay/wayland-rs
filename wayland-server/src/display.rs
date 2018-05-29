use std::env;
use std::ffi::{CStr, CString, OsStr, OsString};
use std::io::{Error as IoError, ErrorKind, Result as IoResult};
use std::os::raw::c_void;
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::os::unix::io::{IntoRawFd, RawFd};
use std::path::PathBuf;
use std::ptr;
use std::rc::Rc;

use wayland_commons::{Implementation, Interface};

#[cfg(feature = "native_lib")]
use globals::global_bind;
use {Client, EventLoop, Global, LoopToken, NewResource};

#[cfg(feature = "native_lib")]
use wayland_sys::server::*;

pub(crate) struct DisplayInner {
    #[cfg(feature = "native_lib")]
    pub(crate) ptr: *mut wl_display,
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

/// The wayland display
///
/// This is the core of your wayland server, this object must
/// be kept alive as long as your server is running. It allows
/// you to manage listening sockets and clients.
pub struct Display {
    inner: Rc<DisplayInner>,
}

impl Display {
    #[cfg(feature = "native_lib")]
    /// Create a new display
    ///
    /// This method provides you a `Display` as well as the main `EventLoop`
    /// which will host your clients' objects.
    ///
    /// Note that at this point, your server is not yet ready to receive connections,
    /// your need to add listening sockets using the `add_socket*` methods.
    pub fn new() -> (Display, EventLoop) {
        let ptr = unsafe { ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_display_create,) };

        let display = Display {
            inner: Rc::new(DisplayInner { ptr: ptr }),
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

    #[cfg(not(feature = "native_lib"))]
    /// Create a new display
    ///
    /// This method provides you a `Display` as well as the main `EventLoop`
    /// which will host your clients' objects.
    ///
    /// Note that at this point, your server is not yet ready to receive connections,
    /// your need to add listening sockets using the `add_socket*` methods.
    pub fn new() -> (Display, EventLoop) {
        unimplemented!()
    }

    /// Create a new global object
    ///
    /// This object will be advertized to all clients, and they will
    /// be able to instanciate it from their registries.
    ///
    /// Your implementation will be called whenever a client instanciates
    /// this global.
    ///
    /// The version specified is the **highest supported version**, you must
    /// be able to handle clients that choose to instanciate this global with
    /// a lower version number.
    pub fn create_global<I: Interface, Impl>(
        &mut self,
        _: &LoopToken,
        version: u32,
        implementation: Impl,
    ) -> Global<I>
    where
        Impl: Implementation<NewResource<I>, u32> + 'static,
    {
        #[cfg(not(feature = "native_lib"))]
        {
            unimplemented!()
        }
        #[cfg(feature = "native_lib")]
        {
            let data = Box::new(Box::new(implementation) as Box<Implementation<NewResource<I>, u32>>);

            unsafe {
                let ptr = ffi_dispatch!(
                    WAYLAND_SERVER_HANDLE,
                    wl_global_create,
                    self.inner.ptr,
                    I::c_interface(),
                    version as i32,
                    &*data as *const Box<_> as *mut _,
                    global_bind::<I>
                );

                Global::create(ptr, data)
            }
        }
    }

    /// Flush events to the clients
    ///
    /// Will send as many pending events as possible to the respective sockets of the clients.
    /// Will not block, but might not send everything if the socket buffer fills up.
    pub fn flush_clients(&self) {
        #[cfg(not(feature = "native_lib"))]
        {
            unimplemented!()
        }
        #[cfg(feature = "native_lib")]
        {
            unsafe { ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_display_flush_clients, self.inner.ptr) };
        }
    }
}

#[cfg(feature = "native_lib")]
unsafe extern "C" fn client_created(_listener: *mut wl_listener, data: *mut c_void) {
    // init the client
    let _client = Client::from_ptr(data as *mut wl_client);
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
        #[cfg(not(feature = "native_lib"))]
        {
            unimplemented!();
        }
        #[cfg(feature = "native_lib")]
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
                    self.inner.ptr,
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
        #[cfg(not(feature = "native_lib"))]
        {
            unimplemented!()
        }
        #[cfg(feature = "native_lib")]
        {
            let ret =
                unsafe { ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_display_add_socket_auto, self.inner.ptr) };
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
        #[cfg(not(feature = "native_lib"))]
        {
            unimplemented!()
        }
        #[cfg(feature = "native_lib")]
        {
            let ret = ffi_dispatch!(
                WAYLAND_SERVER_HANDLE,
                wl_display_add_socket_fd,
                self.inner.ptr,
                fd
            );
            if ret == 0 {
                Ok(())
            } else {
                Err(IoError::new(ErrorKind::InvalidInput, "invalid socket fd"))
            }
        }
    }

    /// Create a new client to this display from an already-existing connected Fd
    pub unsafe fn create_client(&mut self, fd: RawFd) -> Client {
        #[cfg(not(feature = "native_lib"))]
        {
            unimplemented!()
        }
        #[cfg(feature = "native_lib")]
        {
            let ret = ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_client_create, self.inner.ptr, fd);
            Client::from_ptr(ret)
        }
    }
}

#[cfg(feature = "native_lib")]
impl Display {
    /// Retrieve a pointer from the C lib to this `wl_display`
    pub fn c_ptr(&self) -> *mut wl_display {
        self.inner.ptr
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
