use std::{
    cell::RefCell,
    os::raw::{c_char, c_int, c_void},
    os::unix::{io::RawFd, net::UnixStream, prelude::IntoRawFd},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use scoped_tls::scoped_thread_local;
use smallvec::SmallVec;
use wayland_commons::{
    check_for_signature,
    client::{BackendHandle, ClientBackend, InvalidId, NoWaylandLib, ObjectData, WaylandError},
    core_interfaces::WL_DISPLAY_INTERFACE,
    same_interface, AllowNull, Argument, ArgumentType, Interface, Message, ObjectInfo,
    ProtocolError, ANONYMOUS_INTERFACE,
};

use wayland_sys::{client::*, common::*, ffi_dispatch};

use crate::{free_arrays, RUST_MANAGED};

scoped_thread_local!(static HANDLE: RefCell<&mut Handle>);

#[derive(Debug, Clone)]
pub struct Id {
    id: u32,
    ptr: *mut wl_proxy,
    alive: Option<Arc<AtomicBool>>,
    interface: &'static Interface,
}

unsafe impl Send for Id {}

impl wayland_commons::client::ObjectId for Id {
    fn is_null(&self) -> bool {
        self.ptr.is_null()
    }
}

struct ProxyUserData {
    alive: Arc<AtomicBool>,
    data: Arc<dyn ObjectData<Backend>>,
    interface: &'static Interface,
}

pub struct Handle {
    display: *mut wl_display,
    evq: *mut wl_event_queue,
    display_id: Id,
    last_error: Option<WaylandError>,
    pending_placeholder: Option<(&'static Interface, u32)>,
}

pub struct Backend {
    handle: Handle,
}

impl ClientBackend for Backend {
    type ObjectId = Id;
    type Handle = Handle;
    type InitError = NoWaylandLib;

    fn connect(stream: UnixStream) -> Result<Self, NoWaylandLib> {
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
                display_id: Id {
                    id: 1,
                    ptr: display as *mut wl_proxy,
                    alive: Some(display_alive.clone()),
                    interface: &WL_DISPLAY_INTERFACE,
                },
                last_error: None,
                pending_placeholder: None,
            },
        })
    }

    fn connection_fd(&self) -> RawFd {
        unsafe { ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_get_fd, self.handle.display) }
    }

    fn flush(&mut self) -> Result<(), WaylandError> {
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

    fn dispatch_events(&mut self) -> Result<usize, WaylandError> {
        self.handle.no_last_error()?;
        self.handle.try_read()?;
        self.handle.dispatch_pending()
    }

    fn handle(&mut self) -> &mut Self::Handle {
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

impl BackendHandle<Backend> for Handle {
    fn display_id(&self) -> Id {
        self.display_id.clone()
    }

    fn last_error(&self) -> Option<WaylandError> {
        self.last_error.clone()
    }

    fn info(&self, id: Id) -> Result<ObjectInfo, InvalidId> {
        if !id.alive.as_ref().map(|a| a.load(Ordering::Acquire)).unwrap_or(true) || id.ptr.is_null()
        {
            return Err(InvalidId);
        }

        let version = unsafe { ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_get_version, id.ptr) };

        Ok(ObjectInfo { id: id.id, interface: id.interface, version })
    }

    fn null_id(&mut self) -> Id {
        Id { ptr: std::ptr::null_mut(), interface: &ANONYMOUS_INTERFACE, id: 0, alive: None }
    }

    fn placeholder_id(&mut self, spec: Option<(&'static Interface, u32)>) -> Id {
        self.pending_placeholder = spec;
        Id {
            ptr: std::ptr::null_mut(),
            alive: None,
            id: 0,
            interface: spec.map(|(iface, _)| iface).unwrap_or(&ANONYMOUS_INTERFACE),
        }
    }

    fn send_request(
        &mut self,
        Message { sender_id: id, opcode, args }: Message<Id>,
        data: Option<Arc<dyn ObjectData<Backend>>>,
    ) -> Result<Id, InvalidId> {
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
                        panic!("Wrong placeholder used when sending request {}@{}.{}: expected interface {} but got {}", id.interface.name, id.id, message_desc.name, child_interface.name, iface.name);
                    }
                    if !(version == parent_version) {
                        panic!("Wrong placeholder used when sending request {}@{}.{}: expected version {} but got {}", id.interface.name, id.id, message_desc.name, parent_version, version);
                    }
                }
                Some((iface, version))
            } else {
                if let Some(child_interface) = message_desc.child_interface {
                    Some((child_interface, parent_version))
                } else {
                    panic!("Wrong placeholder used when sending request {}@{}.{}: target interface must be specified for a generic constructor.", id.interface.name, id.id, message_desc.name);
                }
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
                    if !o.ptr.is_null() {
                        if !id.alive.as_ref().map(|a| a.load(Ordering::Acquire)).unwrap_or(true) {
                            unsafe { free_arrays(message_desc.signature, &argument_list) };
                            return Err(InvalidId);
                        }
                        let next_interface = arg_interfaces.next().unwrap();
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
        let child_id = if let Some((child_interface, child_version)) = child_spec {
            let child_alive = Arc::new(AtomicBool::new(true));
            let child_id = Id {
                ptr: ret,
                alive: Some(child_alive.clone()),
                id: unsafe { ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_get_id, ret) },
                interface: child_interface,
            };
            let child_data = if id.alive.is_some() {
                let udata = unsafe {
                    &*(ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_get_user_data, id.ptr)
                        as *mut ProxyUserData)
                };
                data.unwrap_or_else(|| {
                    udata.data.clone().make_child(&ObjectInfo {
                        id: child_id.id,
                        interface: child_interface,
                        version: child_version,
                    })
                })
            } else {
                data.expect(
                    "ObjectData must be provided when creating an object from an external proxy.",
                )
            };
            let child_udata = Box::new(ProxyUserData {
                alive: child_alive,
                data: child_data,
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

    fn get_data(&self, id: Id) -> Result<Arc<dyn ObjectData<Backend>>, InvalidId> {
        if !id.alive.as_ref().map(|a| a.load(Ordering::Acquire)).unwrap_or(false) {
            return Err(InvalidId);
        }
        let udata = unsafe {
            &*(ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_get_user_data, id.ptr)
                as *mut ProxyUserData)
        };
        Ok(udata.data.clone())
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
        SmallVec::<[Argument<Id>; 4]>::with_capacity(message_desc.signature.len());
    let mut arg_interfaces = message_desc.arg_interfaces.iter().copied();
    for (i, typ) in message_desc.signature.iter().enumerate() {
        match typ {
            ArgumentType::Uint => parsed_args.push(Argument::Uint((*args.offset(i as isize)).u)),
            ArgumentType::Int => parsed_args.push(Argument::Int((*args.offset(i as isize)).i)),
            ArgumentType::Fixed => parsed_args.push(Argument::Fixed((*args.offset(i as isize)).f)),
            ArgumentType::Fd => parsed_args.push(Argument::Fd((*args.offset(i as isize)).h)),
            ArgumentType::Array(_) => {
                let array = &*((*args.offset(i as isize)).a);
                let content = std::slice::from_raw_parts(array.data as *mut u8, array.size);
                parsed_args.push(Argument::Array(Box::new(content.into())));
            }
            ArgumentType::Str(_) => {
                let ptr = (*args.offset(i as isize)).s;
                let cstr = std::ffi::CStr::from_ptr(ptr);
                parsed_args.push(Argument::Str(Box::new(cstr.into())));
            }
            ArgumentType::Object(allow_null) => {
                let obj = (*args.offset(i as isize)).o as *mut wl_proxy;
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
                        parsed_args.push(Argument::Object(Id {
                            alive: Some(obj_udata.alive.clone()),
                            ptr: obj,
                            id: obj_id,
                            interface: obj_udata.interface,
                        }));
                    } else {
                        parsed_args.push(Argument::Object(Id {
                            alive: None,
                            id: obj_id,
                            ptr: obj,
                            interface: next_interface,
                        }));
                    }
                } else {
                    // libwayland-client.so checks nulls for us
                    parsed_args.push(Argument::Object(Id {
                        alive: None,
                        id: 0,
                        ptr: std::ptr::null_mut(),
                        interface: &ANONYMOUS_INTERFACE,
                    }))
                }
            }
            ArgumentType::NewId(_) => {
                let obj = (*args.offset(i as isize)).o as *mut wl_proxy;
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
                    let child_id = Id {
                        ptr: obj,
                        alive: Some(child_alive.clone()),
                        id: ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_get_id, obj),
                        interface: child_interface,
                    };
                    let child_version =
                        ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_get_version, obj);
                    let child_udata = Box::new(ProxyUserData {
                        alive: child_alive,
                        data: udata.data.clone().make_child(&ObjectInfo {
                            id: child_id.id,
                            interface: child_interface,
                            version: child_version,
                        }),
                        interface: child_interface,
                    });
                    ffi_dispatch!(
                        WAYLAND_CLIENT_HANDLE,
                        wl_proxy_add_dispatcher,
                        obj,
                        dispatcher_func,
                        &RUST_MANAGED as *const u8 as *const c_void,
                        Box::into_raw(child_udata) as *mut c_void
                    );
                    parsed_args.push(Argument::NewId(child_id));
                } else {
                    parsed_args.push(Argument::NewId(Id {
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
    let id = Id {
        alive: Some(udata.alive.clone()),
        ptr: proxy,
        id: proxy_id,
        interface: udata.interface,
    };

    HANDLE.with(|handle| {
        udata.data.event(
            &mut **handle.borrow_mut(),
            Message { sender_id: id.clone(), opcode: opcode as u16, args: parsed_args },
        );
    });

    if message_desc.is_destructor {
        let udata = Box::from_raw(udata_ptr);
        ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_set_user_data, proxy, std::ptr::null_mut());
        udata.alive.store(false, Ordering::Release);
        udata.data.destroyed(id);
        ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_destroy, proxy);
    }

    return 0;
}

extern "C" {
    fn wl_log_trampoline_to_rust_client(fmt: *const c_char, list: *const c_void);
}

#[no_mangle]
pub extern "C" fn wl_log_rust_logger_client(msg: *const c_char) {
    let cstr = unsafe { std::ffi::CStr::from_ptr(msg) };
    let text = cstr.to_string_lossy();
    log::error!("{}", text);
}

impl Drop for Backend {
    fn drop(&mut self) {
        if self.handle.evq.is_null() {
            // we are owning the connection, clone it
            unsafe {
                ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_disconnect, self.handle.display)
            }
        }
    }
}

struct DumbObjectData;

impl ObjectData<Backend> for DumbObjectData {
    fn make_child(self: Arc<Self>, _child_info: &ObjectInfo) -> Arc<dyn ObjectData<Backend>> {
        unreachable!()
    }

    fn event(&self, _handle: &mut Handle, _msg: Message<Id>) {
        unreachable!()
    }

    fn destroyed(&self, _object_id: Id) {
        unreachable!()
    }
}
