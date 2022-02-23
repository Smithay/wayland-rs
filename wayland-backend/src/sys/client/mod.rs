//! Client-side implementation of a Wayland protocol backend using `libwayland`

use std::{
    cell::RefCell,
    ffi::CStr,
    os::raw::{c_char, c_int, c_void},
    os::unix::{io::RawFd, net::UnixStream, prelude::IntoRawFd},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};

use crate::{
    core_interfaces::WL_DISPLAY_INTERFACE,
    protocol::{
        check_for_signature, same_interface, AllowNull, Argument, ArgumentType, Interface, Message,
        ObjectInfo, ProtocolError, ANONYMOUS_INTERFACE,
    },
};
use scoped_tls::scoped_thread_local;
use smallvec::SmallVec;

use wayland_sys::{client::*, common::*, ffi_dispatch};

pub use crate::types::client::{InvalidId, NoWaylandLib, WaylandError};

use super::{free_arrays, RUST_MANAGED};

scoped_thread_local!(static HANDLE: RefCell<&mut Handle>);

/// A trait representing your data associated to an object
///
/// You will only be given access to it as a `&` reference, so you
/// need to handle interior mutability by yourself.
///
/// The methods of this trait will be invoked internally every time a
/// new object is created to initialize its data.
pub trait ObjectData: downcast_rs::DowncastSync {
    /// Dispatch an event for the associated object
    ///
    /// If the event has a NewId argument, the callback must return the object data
    /// for the newly created object
    fn event(
        self: Arc<Self>,
        handle: &mut Handle,
        msg: Message<ObjectId>,
    ) -> Option<Arc<dyn ObjectData>>;
    /// Notification that the object has been destroyed and is no longer active
    fn destroyed(&self, object_id: ObjectId);
    /// Helper for forwarding a Debug implementation of your `ObjectData` type
    ///
    /// By default will just print `ObjectData { ... }`
    fn debug(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ObjectData").finish_non_exhaustive()
    }
}

#[cfg(not(tarpaulin_include))]
impl std::fmt::Debug for dyn ObjectData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.debug(f)
    }
}

downcast_rs::impl_downcast!(sync ObjectData);

/// An ID representing a Wayland object
#[derive(Clone)]
pub struct ObjectId {
    id: u32,
    ptr: *mut wl_proxy,
    alive: Option<Arc<AtomicBool>>,
    interface: &'static Interface,
}

unsafe impl Send for ObjectId {}
unsafe impl Sync for ObjectId {}

impl std::cmp::PartialEq for ObjectId {
    fn eq(&self, other: &ObjectId) -> bool {
        match (&self.alive, &other.alive) {
            (Some(ref a), Some(ref b)) => {
                // this is an object we manage
                Arc::ptr_eq(a, b)
            }
            (None, None) => {
                // this is an external (un-managed) object
                self.ptr == other.ptr
                    && self.id == other.id
                    && same_interface(self.interface, other.interface)
            }
            _ => false,
        }
    }
}

impl std::cmp::Eq for ObjectId {}

impl ObjectId {
    /// Check if this is the null ID
    pub fn is_null(&self) -> bool {
        self.ptr.is_null()
    }

    /// Interface of the represented object
    pub fn interface(&self) -> &'static Interface {
        self.interface
    }

    /// Return the protocol-level numerical ID of this object
    ///
    /// Protocol IDs are reused after object destruction, so this should not be used as a
    /// unique identifier,
    pub fn protocol_id(&self) -> u32 {
        self.id
    }

    /// Creates an object id from a libwayland-client pointer.
    ///
    /// # Errors
    ///
    /// This function returns an [`InvalidId`] error if the interface of the proxy does not match the provided
    /// interface.
    ///
    /// # Safety
    ///
    /// The provided pointer must be a valid pointer to a `wl_resource` and remain valid for as
    /// long as the retrieved `ObjectId` is used.
    pub unsafe fn from_ptr(
        interface: &'static Interface,
        ptr: *mut wl_proxy,
    ) -> Result<ObjectId, InvalidId> {
        let ptr_iface_name =
            CStr::from_ptr(ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_get_class, ptr));
        let provided_iface_name = CStr::from_ptr(
            interface
                .c_ptr
                .expect("[wayland-backend-sys] Cannot use Interface without c_ptr!")
                .name,
        );
        if ptr_iface_name != provided_iface_name {
            return Err(InvalidId);
        }

        let id = ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_get_id, ptr);

        // Test if the proxy is managed by us.
        let is_rust_managed = ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_get_listener, ptr)
            == &RUST_MANAGED as *const u8 as *const _;

        let alive = if is_rust_managed {
            let udata = &*(ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_get_user_data, ptr)
                as *mut ProxyUserData);
            Some(udata.alive.clone())
        } else {
            None
        };

        Ok(ObjectId { id, ptr, alive, interface })
    }

    /// Get the underlying libwayland pointer for this object
    pub fn as_ptr(&self) -> *mut wl_proxy {
        if self.alive.as_ref().map(|alive| alive.load(Ordering::Acquire)).unwrap_or(true) {
            self.ptr
        } else {
            std::ptr::null_mut()
        }
    }
}

#[cfg(not(tarpaulin_include))]
impl std::fmt::Display for ObjectId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}@{}", self.interface.name, self.id)
    }
}

#[cfg(not(tarpaulin_include))]
impl std::fmt::Debug for ObjectId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ObjectId({})", self)
    }
}

struct ProxyUserData {
    alive: Arc<AtomicBool>,
    data: Arc<dyn ObjectData>,
    interface: &'static Interface,
}

/// Main handle of a backend to the Wayland protocol
///
/// This type hosts most of the protocol-related functionality of the backend, and is the
/// main entry point for manipulating Wayland objects. It can be retrieved both from
/// the backend via [`Backend::handle()`](Backend::handle), and is given to you as argument
/// in most event callbacks.
#[derive(Debug)]
pub struct Handle {
    display: *mut wl_display,
    evq: *mut wl_event_queue,
    display_id: ObjectId,
    last_error: Option<WaylandError>,
    pending_placeholder: Option<(&'static Interface, u32)>,
}

/// A pure rust implementation of a Wayland client backend
///
/// This type hosts the plumbing functionalities for interacting with the wayland protocol,
/// and most of the protocol-level interactions are made through the [`Handle`] type, accessed
/// via the [`handle()`](Backend::handle) method.
#[derive(Debug)]
pub struct Backend {
    handle: Handle,
}

unsafe impl Send for Backend {}
unsafe impl Sync for Backend {}

impl Backend {
    /// Try to initialize a Wayland backend on the provided unix stream
    ///
    /// The provided stream should correspond to an already established unix connection with
    /// the Wayland server. This function fails if the system `libwayland` could not be loaded.
    pub fn connect(stream: UnixStream) -> Result<Self, NoWaylandLib> {
        if !is_lib_available() {
            return Err(NoWaylandLib);
        }
        let display = unsafe {
            ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_connect_to_fd, stream.into_raw_fd())
        };
        if display.is_null() {
            panic!("[wayland-backend-sys] libwayland reported an allocation failure.");
        }
        // set the log trampoline
        unsafe {
            ffi_dispatch!(
                WAYLAND_CLIENT_HANDLE,
                wl_log_set_handler_client,
                wl_log_trampoline_to_rust_client
            );
        }
        let display_alive = Arc::new(AtomicBool::new(true));
        Ok(Self {
            handle: Handle {
                display,
                evq: std::ptr::null_mut(),
                display_id: ObjectId {
                    id: 1,
                    ptr: display as *mut wl_proxy,
                    alive: Some(display_alive),
                    interface: &WL_DISPLAY_INTERFACE,
                },
                last_error: None,
                pending_placeholder: None,
            },
        })
    }

    /// Flush all pending outgoing requests to the server
    pub fn flush(&mut self) -> Result<(), WaylandError> {
        self.handle.no_last_error()?;
        let ret =
            unsafe { ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_flush, self.handle.display) };
        if ret < 0 {
            Err(self
                .handle
                .store_if_not_wouldblock_and_return_error(std::io::Error::last_os_error()))
        } else {
            Ok(())
        }
    }

    /// Read events from the wayland socket if available, and invoke the associated callbacks
    ///
    /// This function will never block, and returns an I/O `WouldBlock` error if no event is available
    /// to read.
    ///
    /// **Note:** this function should only be used if you know that you are the only thread
    /// reading events from the wayland socket. If this may not be the case, see [`ReadEventsGuard`]
    pub fn dispatch_events(&mut self) -> Result<usize, WaylandError> {
        self.handle.no_last_error()?;
        self.handle.try_read()?;
        self.handle.dispatch_pending()
    }

    /// Access the [`Handle`] associated with this backend
    pub fn handle(&mut self) -> &mut Handle {
        &mut self.handle
    }
}

impl Handle {
    #[inline]
    fn no_last_error(&self) -> Result<(), WaylandError> {
        if let Some(ref err) = self.last_error {
            Err(err.clone())
        } else {
            Ok(())
        }
    }

    #[inline]
    fn store_and_return_error(&mut self, err: std::io::Error) -> WaylandError {
        // check if it was actually a protocol error
        let err = if err.raw_os_error() == Some(nix::errno::Errno::EPROTO as i32) {
            let mut object_id = 0;
            let mut interface = std::ptr::null();
            let code = unsafe {
                ffi_dispatch!(
                    WAYLAND_CLIENT_HANDLE,
                    wl_display_get_protocol_error,
                    self.display,
                    &mut interface,
                    &mut object_id
                )
            };
            let object_interface = unsafe {
                if interface.is_null() {
                    String::new()
                } else {
                    let cstr = std::ffi::CStr::from_ptr((*interface).name);
                    cstr.to_string_lossy().into()
                }
            };
            WaylandError::Protocol(ProtocolError {
                code,
                object_id,
                object_interface,
                message: String::new(),
            })
        } else {
            WaylandError::Io(err)
        };
        log::error!("{}", err);
        self.last_error = Some(err.clone());
        err
    }

    #[inline]
    fn store_if_not_wouldblock_and_return_error(&mut self, e: std::io::Error) -> WaylandError {
        if e.kind() != std::io::ErrorKind::WouldBlock {
            self.store_and_return_error(e)
        } else {
            e.into()
        }
    }

    fn dispatch_pending(&mut self) -> Result<usize, WaylandError> {
        let display = self.display;
        let evq = self.evq;

        // We erase the lifetime of the Handle to be able to store it in the tls,
        // it's safe as it'll only last until the end of this function call anyway
        let ret =
            HANDLE.set(&RefCell::new(unsafe { std::mem::transmute(&mut *self) }), || unsafe {
                if evq.is_null() {
                    ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_dispatch_pending, display)
                } else {
                    ffi_dispatch!(
                        WAYLAND_CLIENT_HANDLE,
                        wl_display_dispatch_queue_pending,
                        display,
                        evq
                    )
                }
            });
        if ret < 0 {
            Err(self.store_if_not_wouldblock_and_return_error(std::io::Error::last_os_error()))
        } else {
            Ok(ret as usize)
        }
    }

    fn try_read(&mut self) -> Result<(), WaylandError> {
        let ret = unsafe {
            if self.evq.is_null() {
                ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_prepare_read, self.display)
            } else {
                ffi_dispatch!(
                    WAYLAND_CLIENT_HANDLE,
                    wl_display_prepare_read_queue,
                    self.display,
                    self.evq
                )
            }
        };
        if ret < 0 {
            return Ok(());
        }
        let ret =
            unsafe { ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_read_events, self.display) };
        if ret < 0 {
            Err(self.store_if_not_wouldblock_and_return_error(std::io::Error::last_os_error()))
        } else {
            Ok(())
        }
    }
}

/// Guard for synchronizing event reading across multiple threads
///
/// If multiple threads need to read events from the Wayland socket concurrently,
/// it is necessary to synchronize their access. Failing to do so may cause some of the
/// threads to not be notified of new events, and sleep much longer than appropriate.
///
/// To correctly synchronize access, this type should be used. The guard is created using
/// the [`try_new()`](ReadEventsGuard::try_new) method. And the event reading is triggered by consuming
/// the guard using the [`read()`](ReadEventsGuard::read) method.
///
/// If you plan to poll the Wayland socket for readiness, the file descriptor can be retrieved via
/// the [`connection_fd`](ReadEventsGuard::connection_fd) method. Note that for the synchronization to
/// correctly occur, you must *always* create the `ReadEventsGuard` *before* polling the socket.
///
/// This synchronization is compatible with the "prepare_read" mechanism of the system libwayland,
/// and will correctly synchronize with other C libraries using the same Wayland socket.
#[derive(Debug)]
pub struct ReadEventsGuard {
    backend: Arc<Mutex<Backend>>,
    display: *mut wl_display,
    done: bool,
}

impl ReadEventsGuard {
    /// Create a new reading guard
    ///
    /// This call will not block, but event callbacks may be invoked in the process
    /// of preparing the guard.
    pub fn try_new(backend: Arc<Mutex<Backend>>) -> Result<Self, WaylandError> {
        let mut backend_guard = backend.lock().unwrap();
        let display = backend_guard.handle.display;
        let evq = backend_guard.handle.evq;

        // do the prepare_read() and dispatch as necessary
        loop {
            let ret = unsafe {
                if evq.is_null() {
                    ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_prepare_read, display)
                } else {
                    ffi_dispatch!(
                        WAYLAND_CLIENT_HANDLE,
                        wl_display_prepare_read_queue,
                        display,
                        evq
                    )
                }
            };
            if ret < 0 {
                backend_guard.handle.dispatch_pending()?;
            } else {
                break;
            }
        }

        std::mem::drop(backend_guard);

        // prepare_read is done, we are ready
        Ok(ReadEventsGuard { backend, display, done: false })
    }

    /// Access the Wayland socket FD for polling
    pub fn connection_fd(&self) -> RawFd {
        unsafe { ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_get_fd, self.display) }
    }

    /// Attempt to read events from the Wayland socket
    ///
    /// If multiple threads have a live reading guard, this method will block until all of them
    /// are either dropped or have their `read()` method invoked, at which point on of the threads
    /// will read events from the socket and invoke the callbacks for the received events. All
    /// threads will then resume their execution.
    ///
    /// This returns the number of dispatched events, or `0` if an other thread handled the dispatching.
    /// If no events are available to read from the socket, this returns a `WouldBlock` IO error.
    pub fn read(mut self) -> Result<usize, WaylandError> {
        self.done = true;
        let ret =
            unsafe { ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_read_events, self.display) };
        if ret < 0 {
            // we have done the reading, and there is an error
            Err(self
                .backend
                .lock()
                .unwrap()
                .handle
                .store_if_not_wouldblock_and_return_error(std::io::Error::last_os_error()))
        } else {
            // the read occured, dispatch pending events
            self.backend.lock().unwrap().handle.dispatch_pending()
        }
    }
}

impl Drop for ReadEventsGuard {
    fn drop(&mut self) {
        if !self.done {
            unsafe {
                ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_cancel_read, self.display);
            }
        }
    }
}

impl Handle {
    /// Get the object ID for the `wl_display`
    pub fn display_id(&self) -> ObjectId {
        self.display_id.clone()
    }

    /// Get the last error that occurred on this backend
    ///
    /// If this returns an error, your Wayland connection is already dead.
    pub fn last_error(&self) -> Option<WaylandError> {
        self.last_error.clone()
    }

    /// Get the detailed information about a wayland object
    ///
    /// Returns an error if the provided object ID is no longer valid.
    pub fn info(&self, id: ObjectId) -> Result<ObjectInfo, InvalidId> {
        if !id.alive.as_ref().map(|a| a.load(Ordering::Acquire)).unwrap_or(true) || id.ptr.is_null()
        {
            return Err(InvalidId);
        }

        let version = if id.id == 1 {
            // special case the display, because libwayland returns a version of 0 for it
            1
        } else {
            unsafe { ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_get_version, id.ptr) }
        };

        Ok(ObjectInfo { id: id.id, interface: id.interface, version })
    }

    /// Create a null object ID
    ///
    /// This object ID is always invalid, and can be used as placeholder.
    pub fn null_id(&mut self) -> ObjectId {
        ObjectId { ptr: std::ptr::null_mut(), interface: &ANONYMOUS_INTERFACE, id: 0, alive: None }
    }

    /// Create a placeholder ID for object creation
    ///
    /// This ID needs to be created beforehand and given as argument to a request creating a
    /// new object ID. A specification must be specified if the interface and version cannot
    /// be inferred from the protocol (for example object creation from the `wl_registry`).
    ///
    /// If a specification is provided it'll be checked against what can be deduced from the
    /// protocol specification, and [`send_request`](Handle::send_request) will panic if they
    /// do not match.
    pub fn placeholder_id(&mut self, spec: Option<(&'static Interface, u32)>) -> ObjectId {
        self.pending_placeholder = spec;
        ObjectId {
            ptr: std::ptr::null_mut(),
            alive: None,
            id: 0,
            interface: spec.map(|(iface, _)| iface).unwrap_or(&ANONYMOUS_INTERFACE),
        }
    }

    /// Sends a request to the server
    ///
    /// Returns an error if the sender ID of the provided message is no longer valid.
    ///
    /// **Panic:**
    ///
    /// Several checks against the protocol specification are done, and this method will panic if they do
    /// not pass:
    ///
    /// - the message opcode must be valid for the sender interface
    /// - the argument list must match the prototype for the message associated with this opcode
    /// - if the method creates a new object, a [`placeholder_id()`](Handle::placeholder_id) must be given
    ///   in the argument list, either without a specification, or with a specification that matches the
    ///   interface and version deduced from the protocol rules
    ///
    /// When using the system libwayland backend, the Wayland interfaces must have been generated with the C-ptr
    /// support.
    pub fn send_request(
        &mut self,
        Message { sender_id: id, opcode, args }: Message<ObjectId>,
        data: Option<Arc<dyn ObjectData>>,
    ) -> Result<ObjectId, InvalidId> {
        if !id.alive.as_ref().map(|a| a.load(Ordering::Acquire)).unwrap_or(true) || id.ptr.is_null()
        {
            return Err(InvalidId);
        }
        let parent_version = if id.id == 1 {
            1
        } else {
            unsafe { ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_get_version, id.ptr) }
        };
        // check that the argument list is valid
        let message_desc = match id.interface.requests.get(opcode as usize) {
            Some(msg) => msg,
            None => {
                panic!("Unknown opcode {} for object {}@{}.", opcode, id.interface.name, id.id);
            }
        };
        if !check_for_signature(message_desc.signature, &args) {
            panic!(
                "Unexpected signature for request {}@{}.{}: expected {:?}, got {:?}.",
                id.interface.name, id.id, message_desc.name, message_desc.signature, args
            );
        }

        // Prepare the child object data
        let child_spec = if message_desc
            .signature
            .iter()
            .any(|arg| matches!(arg, ArgumentType::NewId(_)))
        {
            if let Some((iface, version)) = self.pending_placeholder.take() {
                if let Some(child_interface) = message_desc.child_interface {
                    if !same_interface(child_interface, iface) {
                        panic!(
                            "Wrong placeholder used when sending request {}@{}.{}: expected interface {} but got {}",
                            id.interface.name,
                            id.id,
                            message_desc.name,
                            child_interface.name,
                            iface.name
                        );
                    }
                    if version != parent_version {
                        panic!(
                            "Wrong placeholder used when sending request {}@{}.{}: expected version {} but got {}",
                            id.interface.name,
                            id.id,
                            message_desc.name,
                            parent_version,
                            version
                        );
                    }
                }
                Some((iface, version))
            } else if let Some(child_interface) = message_desc.child_interface {
                Some((child_interface, parent_version))
            } else {
                panic!(
                    "Wrong placeholder used when sending request {}@{}.{}: target interface must be specified for a generic constructor.",
                    id.interface.name,
                    id.id,
                    message_desc.name
                );
            }
        } else {
            None
        };

        let child_interface_ptr = child_spec
            .as_ref()
            .map(|(i, _)| {
                i.c_ptr.expect("[wayland-backend-sys] Cannot use Interface without c_ptr!")
                    as *const _
            })
            .unwrap_or(std::ptr::null());
        let child_version = child_spec.as_ref().map(|(_, v)| *v).unwrap_or(parent_version);

        // check that all input objects are valid and create the [wl_argument]
        let mut argument_list = SmallVec::<[wl_argument; 4]>::with_capacity(args.len());
        let mut arg_interfaces = message_desc.arg_interfaces.iter();
        for (i, arg) in args.iter().enumerate() {
            match *arg {
                Argument::Uint(u) => argument_list.push(wl_argument { u }),
                Argument::Int(i) => argument_list.push(wl_argument { i }),
                Argument::Fixed(f) => argument_list.push(wl_argument { f }),
                Argument::Fd(h) => argument_list.push(wl_argument { h }),
                Argument::Array(ref a) => {
                    let a = Box::new(wl_array {
                        size: a.len(),
                        alloc: a.len(),
                        data: a.as_ptr() as *mut _,
                    });
                    argument_list.push(wl_argument { a: Box::into_raw(a) })
                }
                Argument::Str(ref s) => argument_list.push(wl_argument { s: s.as_ptr() }),
                Argument::Object(ref o) => {
                    let next_interface = arg_interfaces.next().unwrap();
                    if !o.ptr.is_null() {
                        if !id.alive.as_ref().map(|a| a.load(Ordering::Acquire)).unwrap_or(true) {
                            unsafe { free_arrays(message_desc.signature, &argument_list) };
                            return Err(InvalidId);
                        }
                        if !same_interface(next_interface, o.interface) {
                            panic!("Request {}@{}.{} expects an argument of interface {} but {} was provided instead.", id.interface.name, id.id, message_desc.name, next_interface.name, o.interface.name);
                        }
                    } else if !matches!(
                        message_desc.signature[i],
                        ArgumentType::Object(AllowNull::Yes)
                    ) {
                        panic!(
                            "Request {}@{}.{} expects an non-null object argument.",
                            id.interface.name, id.id, message_desc.name
                        );
                    }
                    argument_list.push(wl_argument { o: o.ptr as *const _ })
                }
                Argument::NewId(_) => argument_list.push(wl_argument { n: 0 }),
            }
        }

        let ret = unsafe {
            ffi_dispatch!(
                WAYLAND_CLIENT_HANDLE,
                wl_proxy_marshal_array_constructor_versioned,
                id.ptr,
                opcode as u32,
                argument_list.as_mut_ptr(),
                child_interface_ptr,
                child_version
            )
        };

        unsafe {
            free_arrays(message_desc.signature, &argument_list);
        }

        if ret.is_null() && child_spec.is_some() {
            panic!("[wayland-backend-sys] libwayland reported an allocation failure.");
        }

        // initialize the proxy
        let child_id = if let Some((child_interface, _)) = child_spec {
            let child_alive = Arc::new(AtomicBool::new(true));
            let child_id = ObjectId {
                ptr: ret,
                alive: Some(child_alive.clone()),
                id: unsafe { ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_get_id, ret) },
                interface: child_interface,
            };
            let child_udata = Box::new(ProxyUserData {
                alive: child_alive,
                data: data.expect(
                    "Sending a request creating an object without providing an object data.",
                ),
                interface: child_interface,
            });
            unsafe {
                ffi_dispatch!(
                    WAYLAND_CLIENT_HANDLE,
                    wl_proxy_add_dispatcher,
                    ret,
                    dispatcher_func,
                    &RUST_MANAGED as *const u8 as *const c_void,
                    Box::into_raw(child_udata) as *mut c_void
                );
            }
            child_id
        } else {
            self.null_id()
        };

        if message_desc.is_destructor {
            if let Some(ref alive) = id.alive {
                let udata = unsafe {
                    Box::from_raw(ffi_dispatch!(
                        WAYLAND_CLIENT_HANDLE,
                        wl_proxy_get_user_data,
                        id.ptr
                    ) as *mut ProxyUserData)
                };
                unsafe {
                    ffi_dispatch!(
                        WAYLAND_CLIENT_HANDLE,
                        wl_proxy_set_user_data,
                        id.ptr,
                        std::ptr::null_mut()
                    );
                }
                alive.store(false, Ordering::Release);
                udata.data.destroyed(id.clone());
            }
            unsafe {
                ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_destroy, id.ptr);
            }
        }

        Ok(child_id)
    }

    /// Access the object data associated with a given object ID
    ///
    /// Returns an error if the object ID is not longer valid or if it corresponds to a Wayland
    /// object that is not managed by this backend (when multiple libraries share the same Wayland
    /// socket via `libwayland`).
    pub fn get_data(&self, id: ObjectId) -> Result<Arc<dyn ObjectData>, InvalidId> {
        if !id.alive.as_ref().map(|a| a.load(Ordering::Acquire)).unwrap_or(false) {
            return Err(InvalidId);
        }

        if id.id == 1 {
            // special case the display whose object data is not accessible
            return Ok(Arc::new(DumbObjectData));
        }

        let udata = unsafe {
            &*(ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_get_user_data, id.ptr)
                as *mut ProxyUserData)
        };
        Ok(udata.data.clone())
    }

    /// Set the object data associated with a given object ID
    ///
    /// Returns an error if the object ID is not longer valid or if it corresponds to a Wayland
    /// object that is not managed by this backend (when multiple libraries share the same Wayland
    /// socket via `libwayland`).
    pub fn set_data(&mut self, id: ObjectId, data: Arc<dyn ObjectData>) -> Result<(), InvalidId> {
        if !id.alive.as_ref().map(|a| a.load(Ordering::Acquire)).unwrap_or(false) {
            return Err(InvalidId);
        }

        // Cannot touch the user_data of the display
        if id.id == 1 {
            return Err(InvalidId);
        }

        let udata = unsafe {
            &mut *(ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_get_user_data, id.ptr)
                as *mut ProxyUserData)
        };

        udata.data = data;

        Ok(())
    }
}

unsafe extern "C" fn dispatcher_func(
    _: *const c_void,
    proxy: *mut c_void,
    opcode: u32,
    _: *const wl_message,
    args: *const wl_argument,
) -> c_int {
    let proxy = proxy as *mut wl_proxy;
    let udata_ptr =
        ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_get_user_data, proxy) as *mut ProxyUserData;
    let udata = &mut *udata_ptr;
    let interface = udata.interface;
    let message_desc = match interface.events.get(opcode as usize) {
        Some(desc) => desc,
        None => {
            log::error!("Unknown event opcode {} for interface {}.", opcode, interface.name);
            return -1;
        }
    };

    let mut parsed_args =
        SmallVec::<[Argument<ObjectId>; 4]>::with_capacity(message_desc.signature.len());
    let mut arg_interfaces = message_desc.arg_interfaces.iter().copied();
    let mut created = None;
    for (i, typ) in message_desc.signature.iter().enumerate() {
        match typ {
            ArgumentType::Uint => parsed_args.push(Argument::Uint((*args.add(i)).u)),
            ArgumentType::Int => parsed_args.push(Argument::Int((*args.add(i)).i)),
            ArgumentType::Fixed => parsed_args.push(Argument::Fixed((*args.add(i)).f)),
            ArgumentType::Fd => parsed_args.push(Argument::Fd((*args.add(i)).h)),
            ArgumentType::Array(_) => {
                let array = &*((*args.add(i)).a);
                let content = std::slice::from_raw_parts(array.data as *mut u8, array.size);
                parsed_args.push(Argument::Array(Box::new(content.into())));
            }
            ArgumentType::Str(_) => {
                let ptr = (*args.add(i)).s;
                let cstr = std::ffi::CStr::from_ptr(ptr);
                parsed_args.push(Argument::Str(Box::new(cstr.into())));
            }
            ArgumentType::Object(_) => {
                let obj = (*args.add(i)).o as *mut wl_proxy;
                if !obj.is_null() {
                    // retrieve the object relevant info
                    let obj_id = ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_get_id, obj);
                    // check if this is a local or distant proxy
                    let next_interface = arg_interfaces.next().unwrap_or(&ANONYMOUS_INTERFACE);
                    let listener = ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_get_listener, obj);
                    if listener == &RUST_MANAGED as *const u8 as *const c_void {
                        let obj_udata =
                            &*(ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_get_user_data, obj)
                                as *mut ProxyUserData);
                        if !same_interface(next_interface, obj_udata.interface) {
                            log::error!(
                                "Received object {}@{} in {}.{} but expected interface {}.",
                                obj_udata.interface.name,
                                obj_id,
                                interface.name,
                                message_desc.name,
                                next_interface.name,
                            );
                            return -1;
                        }
                        parsed_args.push(Argument::Object(ObjectId {
                            alive: Some(obj_udata.alive.clone()),
                            ptr: obj,
                            id: obj_id,
                            interface: obj_udata.interface,
                        }));
                    } else {
                        parsed_args.push(Argument::Object(ObjectId {
                            alive: None,
                            id: obj_id,
                            ptr: obj,
                            interface: next_interface,
                        }));
                    }
                } else {
                    // libwayland-client.so checks nulls for us
                    parsed_args.push(Argument::Object(ObjectId {
                        alive: None,
                        id: 0,
                        ptr: std::ptr::null_mut(),
                        interface: &ANONYMOUS_INTERFACE,
                    }))
                }
            }
            ArgumentType::NewId(_) => {
                let obj = (*args.add(i)).o as *mut wl_proxy;
                // this is a newid, it needs to be initialized
                if !obj.is_null() {
                    let child_interface = message_desc.child_interface.unwrap_or_else(|| {
                        log::warn!(
                            "Event {}.{} creates an anonymous object.",
                            interface.name,
                            opcode
                        );
                        &ANONYMOUS_INTERFACE
                    });
                    let child_alive = Arc::new(AtomicBool::new(true));
                    let child_id = ObjectId {
                        ptr: obj,
                        alive: Some(child_alive.clone()),
                        id: ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_get_id, obj),
                        interface: child_interface,
                    };
                    let child_udata = Box::into_raw(Box::new(ProxyUserData {
                        alive: child_alive,
                        data: Arc::new(UninitObjectData),
                        interface: child_interface,
                    }));
                    created = Some((child_id.clone(), child_udata));
                    ffi_dispatch!(
                        WAYLAND_CLIENT_HANDLE,
                        wl_proxy_add_dispatcher,
                        obj,
                        dispatcher_func,
                        &RUST_MANAGED as *const u8 as *const c_void,
                        child_udata as *mut c_void
                    );
                    parsed_args.push(Argument::NewId(child_id));
                } else {
                    parsed_args.push(Argument::NewId(ObjectId {
                        id: 0,
                        ptr: std::ptr::null_mut(),
                        alive: None,
                        interface: &ANONYMOUS_INTERFACE,
                    }))
                }
            }
        }
    }

    let proxy_id = ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_get_id, proxy);
    let id = ObjectId {
        alive: Some(udata.alive.clone()),
        ptr: proxy,
        id: proxy_id,
        interface: udata.interface,
    };

    let ret = HANDLE.with(|handle| {
        udata.data.clone().event(
            &mut **handle.borrow_mut(),
            Message { sender_id: id.clone(), opcode: opcode as u16, args: parsed_args },
        )
    });

    if message_desc.is_destructor {
        let udata = Box::from_raw(udata_ptr);
        ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_set_user_data, proxy, std::ptr::null_mut());
        udata.alive.store(false, Ordering::Release);
        udata.data.destroyed(id);
        ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_destroy, proxy);
    }

    match (created, ret) {
        (Some((_, child_udata_ptr)), Some(child_data)) => {
            (*child_udata_ptr).data = child_data;
        }
        (Some((child_id, _)), None) => {
            panic!("Callback creating object {} did not provide any object data.", child_id);
        }
        (None, Some(_)) => {
            panic!("An object data was returned from a callback not creating any object");
        }
        (None, None) => {}
    }

    0
}

extern "C" {
    fn wl_log_trampoline_to_rust_client(fmt: *const c_char, list: *const c_void);
}

impl Drop for Backend {
    fn drop(&mut self) {
        if self.handle.evq.is_null() {
            // we are own the connection, clone it
            unsafe {
                ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_disconnect, self.handle.display)
            }
        }
    }
}

struct DumbObjectData;

impl ObjectData for DumbObjectData {
    fn event(
        self: Arc<Self>,
        _handle: &mut Handle,
        _msg: Message<ObjectId>,
    ) -> Option<Arc<dyn ObjectData>> {
        unreachable!()
    }

    fn destroyed(&self, _object_id: ObjectId) {
        unreachable!()
    }
}

struct UninitObjectData;

impl ObjectData for UninitObjectData {
    fn event(
        self: Arc<Self>,
        _handle: &mut Handle,
        msg: Message<ObjectId>,
    ) -> Option<Arc<dyn ObjectData>> {
        panic!("Received a message on an uninitialized object: {:?}", msg);
    }

    fn destroyed(&self, _object_id: ObjectId) {}

    fn debug(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UninitObjectData").finish()
    }
}
