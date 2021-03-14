use std::{
    cell::RefCell,
    ffi::{CStr, CString},
    os::raw::{c_int, c_void},
    os::unix::{
        io::{IntoRawFd, RawFd},
        net::UnixStream,
    },
    ptr::null_mut,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use scoped_tls::scoped_thread_local;
use smallvec::SmallVec;
use wayland_commons::{
    check_for_signature,
    core_interfaces::WL_DISPLAY_INTERFACE,
    same_interface,
    server::{
        BackendHandle, ClientData, CommonPollBackend, DisconnectReason, GlobalHandler, GlobalInfo,
        InvalidId, NoWaylandLib, ObjectData, ServerBackend,
    },
    AllowNull, Argument, ArgumentType, Interface, ObjectInfo, ANONYMOUS_INTERFACE,
};

use wayland_sys::{common::*, ffi_dispatch, server::*};

use crate::{free_arrays, RUST_MANAGED};

// First pointer is &mut Handle<D>, and second pointer is &mut D
scoped_thread_local!(static HANDLE: (*mut c_void, *mut c_void));

#[derive(Debug, Clone)]
pub struct ObjectId {
    id: u32,
    ptr: *mut wl_resource,
    alive: Option<Arc<AtomicBool>>,
    interface: &'static Interface,
}

unsafe impl Send for ObjectId {}

impl wayland_commons::server::ObjectId for ObjectId {
    fn is_null(&self) -> bool {
        self.ptr.is_null()
    }
}

#[derive(Debug, Clone)]
pub struct ClientId {
    ptr: *mut wl_client,
    alive: Arc<AtomicBool>,
}

unsafe impl Send for ClientId {}

#[derive(Debug, Clone)]
pub struct GlobalId {
    ptr: *mut wl_global,
    alive: Arc<AtomicBool>,
}

unsafe impl Send for GlobalId {}

struct ResourceUserData<D> {
    alive: Arc<AtomicBool>,
    data: Arc<dyn ObjectData<D, Backend<D>>>,
    interface: &'static Interface,
}

struct ClientUserData<D> {
    data: Arc<dyn ClientData<D, Backend<D>>>,
    alive: Arc<AtomicBool>,
}

struct GlobalUserData<D> {
    handler: Arc<dyn GlobalHandler<D, Backend<D>>>,
    interface: &'static Interface,
    version: u32,
    disabled: bool,
    alive: Arc<AtomicBool>,
    ptr: *mut wl_global,
}

pub struct Handle<D> {
    display: *mut wl_display,
    _data: std::marker::PhantomData<fn(&mut D)>,
}

pub struct Backend<D> {
    handle: Handle<D>,
}

impl<D> ServerBackend<D> for Backend<D> {
    type ObjectId = ObjectId;
    type GlobalId = GlobalId;
    type ClientId = ClientId;
    type InitError = NoWaylandLib;
    type Handle = Handle<D>;

    fn new() -> Result<Self, NoWaylandLib> {
        if !is_lib_available() {
            return Err(NoWaylandLib);
        }

        let display = unsafe { ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_display_create,) };
        if display.is_null() {
            panic!("[wayland-backend-sys] libwayland reported an allocation failure.");
        }

        unsafe {
            ffi_dispatch!(
                WAYLAND_SERVER_HANDLE,
                wl_display_set_global_filter,
                display,
                global_filter::<D>,
                std::ptr::null_mut()
            );
        }

        Ok(Backend { handle: Handle { display, _data: std::marker::PhantomData } })
    }

    fn insert_client(
        &mut self,
        stream: UnixStream,
        data: Arc<dyn ClientData<D, Self>>,
    ) -> std::io::Result<Self::ClientId> {
        let ret = unsafe {
            ffi_dispatch!(
                WAYLAND_SERVER_HANDLE,
                wl_client_create,
                self.handle.display,
                stream.into_raw_fd()
            )
        };

        if ret.is_null() {
            return Err(std::io::Error::last_os_error());
        }

        Ok(unsafe { init_client::<D>(ret, data) })
    }

    fn flush(&mut self, client: Option<Self::ClientId>) -> std::io::Result<()> {
        if let Some(client_id) = client {
            if client_id.alive.load(Ordering::Acquire) {
                unsafe { ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_client_flush, client_id.ptr) };
            }
        } else {
            unsafe {
                ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_display_flush_clients, self.handle.display);
            }
        }
        Ok(())
    }

    fn handle(&mut self) -> &mut Self::Handle {
        &mut self.handle
    }
}

impl<D> CommonPollBackend<D> for Backend<D> {
    fn poll_fd(&self) -> RawFd {
        unsafe {
            let evl_ptr = ffi_dispatch!(
                WAYLAND_SERVER_HANDLE,
                wl_display_get_event_loop,
                self.handle.display
            );
            ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_event_loop_get_fd, evl_ptr)
        }
    }

    fn dispatch_events(&mut self, data: &mut D) -> std::io::Result<usize> {
        let display = self.handle.display;
        let pointers = (&mut self.handle as *mut _ as *mut c_void, data as *mut _ as *mut c_void);
        let ret = HANDLE.set(&pointers, || unsafe {
            let evl_ptr = ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_display_get_event_loop, display);
            ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_event_loop_dispatch, evl_ptr, 0)
        });

        if ret < 0 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(ret as usize)
        }
    }
}

impl<D> BackendHandle<D, Backend<D>> for Handle<D> {
    fn object_info(&self, id: ObjectId) -> Result<ObjectInfo, InvalidId> {
        if !id.alive.as_ref().map(|alive| alive.load(Ordering::Acquire)).unwrap_or(true) {
            return Err(InvalidId);
        }

        let version =
            unsafe { ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_resource_get_version, id.ptr) } as u32;

        Ok(ObjectInfo { id: id.id, version, interface: id.interface })
    }

    fn get_client(&self, id: ObjectId) -> Result<ClientId, InvalidId> {
        if !id.alive.map(|alive| alive.load(Ordering::Acquire)).unwrap_or(true) {
            return Err(InvalidId);
        }

        unsafe {
            let client_ptr = ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_resource_get_client, id.ptr);
            client_id_from_ptr::<D>(client_ptr).ok_or(InvalidId)
        }
    }

    fn get_client_data(
        &self,
        id: ClientId,
    ) -> Result<Arc<dyn ClientData<D, Backend<D>>>, InvalidId> {
        if !id.alive.load(Ordering::Acquire) {
            return Err(InvalidId);
        }

        let data = unsafe {
            match client_user_data::<D>(id.ptr) {
                Some(ptr) => &mut *ptr,
                None => return Err(InvalidId),
            }
        };

        Ok(data.data.clone())
    }

    fn all_clients<'a>(&'a self) -> Box<dyn Iterator<Item = ClientId> + 'a> {
        let mut client_list = unsafe {
            ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_display_get_client_list, self.display)
        };
        Box::new(std::iter::from_fn(move || {
            if client_list.is_null() {
                None
            } else {
                unsafe {
                    let client =
                        ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_client_from_link, client_list);
                    let id = client_id_from_ptr::<D>(client);

                    let next = (*client_list).next;
                    if client_list == next {
                        client_list = std::ptr::null_mut();
                    } else {
                        client_list = next;
                    }

                    id
                }
            }
        }))
    }

    fn all_objects_for<'a>(
        &'a self,
        client_id: ClientId,
    ) -> Result<Box<dyn Iterator<Item = ObjectId> + 'a>, InvalidId> {
        todo!()
    }

    fn create_object(
        &mut self,
        client: ClientId,
        interface: &'static Interface,
        version: u32,
        data: Arc<dyn ObjectData<D, Backend<D>>>,
    ) -> Result<ObjectId, InvalidId> {
        if !client.alive.load(Ordering::Acquire) {
            return Err(InvalidId);
        }

        let interface_ptr =
            interface.c_ptr.expect("Interface without c_ptr are unsupported by the sys backend.");

        let resource = unsafe {
            ffi_dispatch!(
                WAYLAND_SERVER_HANDLE,
                wl_resource_create,
                client.ptr,
                interface_ptr,
                version as i32,
                0
            )
        };

        Ok(unsafe { init_resource(resource, interface, data) })
    }

    fn null_id(&mut self) -> ObjectId {
        ObjectId { ptr: std::ptr::null_mut(), id: 0, alive: None, interface: &ANONYMOUS_INTERFACE }
    }

    fn send_event(
        &mut self,
        id: ObjectId,
        opcode: u16,
        args: &[Argument<ObjectId>],
    ) -> Result<(), InvalidId> {
        if !id.alive.as_ref().map(|a| a.load(Ordering::Acquire)).unwrap_or(true) || id.ptr.is_null()
        {
            return Err(InvalidId);
        }

        // check that the argument list is valid
        let message_desc = match id.interface.events.get(opcode as usize) {
            Some(msg) => msg,
            None => {
                panic!("Unknown opcode {} for object {}@{}.", opcode, id.interface.name, id.id);
            }
        };
        if !check_for_signature(message_desc.signature, args) {
            panic!(
                "Unexpected signature for request {}@{}.{}: expected {:?}, got {:?}.",
                id.interface.name, id.id, message_desc.name, message_desc.signature, args
            );
        }

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
                        // check that the object belongs to the right client
                        if !(self.get_client(id.clone()).unwrap().ptr
                            == self.get_client(o.clone()).unwrap().ptr)
                        {
                            panic!("Attempting to send an event with objects from wrong client.");
                        }
                        let next_interface = arg_interfaces.next().unwrap();
                        if !same_interface(next_interface, o.interface) {
                            panic!("Event {}@{}.{} expects an argument of interface {} but {} was provided instead.", id.interface.name, id.id, message_desc.name, next_interface.name, o.interface.name);
                        }
                    } else if !matches!(
                        message_desc.signature[i],
                        ArgumentType::Object(AllowNull::Yes)
                    ) {
                        panic!(
                            "Event {}@{}.{} expects an non-null object argument.",
                            id.interface.name, id.id, message_desc.name
                        );
                    }
                    argument_list.push(wl_argument { o: o.ptr as *const _ })
                }
                Argument::NewId(ref o) => {
                    if !o.ptr.is_null() {
                        if !id.alive.as_ref().map(|a| a.load(Ordering::Acquire)).unwrap_or(true) {
                            unsafe { free_arrays(message_desc.signature, &argument_list) };
                            return Err(InvalidId);
                        }
                        // check that the object belongs to the right client
                        if !(self.get_client(id.clone()).unwrap().ptr
                            == self.get_client(o.clone()).unwrap().ptr)
                        {
                            panic!("Attempting to send an event with objects from wrong client.");
                        }
                        let child_interface = match message_desc.child_interface {
                            Some(iface) => iface,
                            None => panic!("Trying to send event {}@{}.{} which creates an object without specifying its interface, this is unsupported.", id.interface.name, id.id, message_desc.name),
                        };
                        if !same_interface(child_interface, o.interface) {
                            panic!("Event {}@{}.{} expects an argument of interface {} but {} was provided instead.", id.interface.name, id.id, message_desc.name, child_interface.name, o.interface.name);
                        }
                    } else if !matches!(
                        message_desc.signature[i],
                        ArgumentType::NewId(AllowNull::Yes)
                    ) {
                        panic!(
                            "Event {}@{}.{} expects an non-null object argument.",
                            id.interface.name, id.id, message_desc.name
                        );
                    }
                    argument_list.push(wl_argument { o: o.ptr as *const _ })
                }
            }
        }

        unsafe {
            ffi_dispatch!(
                WAYLAND_SERVER_HANDLE,
                wl_resource_post_event_array,
                id.ptr,
                opcode as u32,
                argument_list.as_mut_ptr()
            );
        }

        unsafe {
            free_arrays(message_desc.signature, &argument_list);
        }

        if message_desc.is_destructor {
            if let Some(ref alive) = id.alive {
                let udata = unsafe {
                    Box::from_raw(ffi_dispatch!(
                        WAYLAND_SERVER_HANDLE,
                        wl_resource_get_user_data,
                        id.ptr
                    ) as *mut ResourceUserData<D>)
                };
                unsafe {
                    ffi_dispatch!(
                        WAYLAND_SERVER_HANDLE,
                        wl_resource_set_user_data,
                        id.ptr,
                        std::ptr::null_mut()
                    );
                }
                alive.store(false, Ordering::Release);
                udata.data.destroyed(self.get_client(id.clone()).unwrap(), id.clone());
            }
            unsafe {
                ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_resource_destroy, id.ptr);
            }
        }

        Ok(())
    }

    fn get_object_data(
        &self,
        id: ObjectId,
    ) -> Result<Arc<dyn ObjectData<D, Backend<D>>>, InvalidId> {
        if !id.alive.as_ref().map(|alive| alive.load(Ordering::Acquire)).unwrap_or(false) {
            return Err(InvalidId);
        }

        let udata = unsafe {
            &mut *(ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_resource_get_user_data, id.ptr)
                as *mut ResourceUserData<D>)
        };

        Ok(udata.data.clone())
    }

    fn post_error(&mut self, id: ObjectId, error_code: u32, message: CString) {
        if !id.alive.as_ref().map(|alive| alive.load(Ordering::Acquire)).unwrap_or(true) {
            return;
        }

        unsafe {
            ffi_dispatch!(
                WAYLAND_SERVER_HANDLE,
                wl_resource_post_error,
                id.ptr,
                error_code,
                message.as_ptr()
            )
        }
    }

    fn kill_client(&mut self, client_id: ClientId, reason: DisconnectReason) {
        if !client_id.alive.load(Ordering::Acquire) {
            return;
        }
        if let Some(udata) = unsafe { client_user_data::<D>(client_id.ptr) } {
            let udata = unsafe { &*udata };
            udata.alive.store(false, Ordering::Release);
            udata.data.disconnected(client_id.clone(), reason);
        }

        unsafe {
            ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_client_destroy, client_id.ptr);
        }
    }

    fn create_global(
        &mut self,
        interface: &'static Interface,
        version: u32,
        handler: Arc<dyn GlobalHandler<D, Backend<D>>>,
    ) -> GlobalId {
        let alive = Arc::new(AtomicBool::new(true));

        let interface_ptr =
            interface.c_ptr.expect("Interface without c_ptr are unsupported by the sys backend.");

        let udata = Box::into_raw(Box::new(GlobalUserData {
            handler,
            alive: alive.clone(),
            interface,
            version,
            disabled: false,
            ptr: std::ptr::null_mut(),
        }));

        let ret = unsafe {
            ffi_dispatch!(
                WAYLAND_SERVER_HANDLE,
                wl_global_create,
                self.display,
                interface_ptr,
                version as i32,
                udata as *mut c_void,
                global_bind::<D>
            )
        };

        if ret.is_null() {
            panic!(
                "[wayland-backend-sys] Invalid global specification or memory allocation failure."
            );
        }

        unsafe {
            (*udata).ptr = ret;
        }

        GlobalId { ptr: ret, alive }
    }

    fn disable_global(&mut self, id: GlobalId) {
        if !id.alive.load(Ordering::Acquire) {
            return;
        }

        let udata = unsafe {
            &mut *(ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_global_get_user_data, id.ptr)
                as *mut GlobalUserData<D>)
        };
        udata.disabled = true;

        unsafe {
            ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_global_remove, id.ptr);
        }
    }

    fn remove_global(&mut self, id: GlobalId) {
        if !id.alive.load(Ordering::Acquire) {
            return;
        }

        let udata = unsafe {
            Box::from_raw(ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_global_get_user_data, id.ptr)
                as *mut GlobalUserData<D>)
        };
        udata.alive.store(false, Ordering::Release);

        unsafe {
            ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_global_destroy, id.ptr);
        }
    }

    fn global_info(&self, id: GlobalId) -> Result<GlobalInfo, InvalidId> {
        if !id.alive.load(Ordering::Acquire) {
            return Err(InvalidId);
        }
        let udata = unsafe {
            &*(ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_global_get_user_data, id.ptr)
                as *mut GlobalUserData<D>)
        };

        Ok(GlobalInfo {
            interface: udata.interface,
            version: udata.version,
            disabled: udata.disabled,
        })
    }

    fn get_global_handler(
        &self,
        id: GlobalId,
    ) -> Result<Arc<dyn GlobalHandler<D, Backend<D>>>, InvalidId> {
        if !id.alive.load(Ordering::Acquire) {
            return Err(InvalidId);
        }

        let udata = unsafe {
            Box::from_raw(ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_global_get_user_data, id.ptr)
                as *mut GlobalUserData<D>)
        };
        Ok(udata.handler.clone())
    }
}

unsafe fn init_client<D>(
    client: *mut wl_client,
    data: Arc<dyn ClientData<D, Backend<D>>>,
) -> ClientId {
    let alive = Arc::new(AtomicBool::new(true));
    let client_data = Box::into_raw(Box::new(ClientUserData { alive: alive.clone(), data }));

    let listener = signal::rust_listener_create(client_destroy_notify::<D>);
    signal::rust_listener_set_user_data(listener, client_data as *mut c_void);

    ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_client_add_destroy_listener, client, listener);

    ClientId { ptr: client, alive }
}

unsafe fn client_id_from_ptr<D>(client: *mut wl_client) -> Option<ClientId> {
    if let Some(udata) = client_user_data::<D>(client) {
        Some(ClientId { ptr: client, alive: (*udata).alive.clone() })
    } else {
        None
    }
}

unsafe fn client_user_data<D>(client: *mut wl_client) -> Option<*mut ClientUserData<D>> {
    if client.is_null() {
        return None;
    }
    let listener = ffi_dispatch!(
        WAYLAND_SERVER_HANDLE,
        wl_client_get_destroy_listener,
        client,
        client_destroy_notify::<D>
    );
    if !listener.is_null() {
        Some(signal::rust_listener_get_user_data(listener) as *mut ClientUserData<D>)
    } else {
        None
    }
}

unsafe extern "C" fn client_destroy_notify<D>(listener: *mut wl_listener, client_ptr: *mut c_void) {
    let data =
        Box::from_raw(signal::rust_listener_get_user_data(listener) as *mut ClientUserData<D>);
    signal::rust_listener_destroy(listener);
    // only notify the killing if it was not already
    if data.alive.load(Ordering::Acquire) {
        data.alive.store(false, Ordering::Release);
        data.data.disconnected(
            ClientId { ptr: client_ptr as *mut wl_client, alive: data.alive.clone() },
            DisconnectReason::ConnectionClosed,
        );
    }
}

unsafe extern "C" fn global_bind<D>(
    client: *mut wl_client,
    data: *mut c_void,
    version: u32,
    id: u32,
) {
    let global_udata = &mut *(data as *mut GlobalUserData<D>);

    let global_id = GlobalId { alive: global_udata.alive.clone(), ptr: global_udata.ptr };

    let client_id = match client_id_from_ptr::<D>(client) {
        Some(id) => id,
        None => return,
    };

    // this must be Some(), checked at creation of the global
    let interface_ptr = global_udata.interface.c_ptr.unwrap();

    HANDLE.with(|&(handle_ptr, data_ptr)| {
        let handle = &mut *(handle_ptr as *mut Handle<D>);
        let data = &mut *(data_ptr as *mut D);
        // create the object
        let resource = ffi_dispatch!(
            WAYLAND_SERVER_HANDLE,
            wl_resource_create,
            client,
            interface_ptr,
            version as i32,
            id
        );
        let object_udata = global_udata
            .handler
            .clone()
            .make_data(data, &ObjectInfo { id, interface: global_udata.interface, version });
        let object_id = init_resource(resource, global_udata.interface, object_udata);
        global_udata.handler.bind(handle, data, client_id, global_id, object_id);
    })
}

unsafe extern "C" fn global_filter<D>(
    client: *const wl_client,
    global: *const wl_global,
    _: *mut c_void,
) -> bool {
    let client_id = match client_id_from_ptr::<D>(client as *mut _) {
        Some(id) => id,
        None => return false,
    };

    let global_udata = &*(ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_global_get_user_data, global)
        as *mut GlobalUserData<D>);

    let global_id = GlobalId { ptr: global as *mut wl_global, alive: global_udata.alive.clone() };

    global_udata.handler.can_view(client_id, global_id)
}

unsafe fn init_resource<D>(
    resource: *mut wl_resource,
    interface: &'static Interface,
    data: Arc<dyn ObjectData<D, Backend<D>>>,
) -> ObjectId {
    let alive = Arc::new(AtomicBool::new(true));
    let udata = Box::into_raw(Box::new(ResourceUserData { data, interface, alive: alive.clone() }));
    let id = ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_resource_get_id, resource);

    ffi_dispatch!(
        WAYLAND_SERVER_HANDLE,
        wl_resource_set_dispatcher,
        resource,
        resource_dispatcher::<D>,
        &RUST_MANAGED as *const u8 as *const c_void,
        udata as *mut c_void,
        Some(resource_destructor::<D>)
    );

    ObjectId { interface, alive: Some(alive), id, ptr: resource }
}

unsafe extern "C" fn resource_dispatcher<D>(
    _: *const c_void,
    resource: *mut c_void,
    opcode: u32,
    _: *const wl_message,
    args: *const wl_argument,
) -> i32 {
    let resource = resource as *mut wl_resource;
    let udata_ptr = ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_resource_get_user_data, resource)
        as *mut ResourceUserData<D>;
    let udata = &mut *udata_ptr;
    let client = ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_resource_get_client, resource);
    let resource_id = ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_resource_get_id, resource);
    let version = ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_resource_get_version, resource);
    let interface = udata.interface;
    let message_desc = match interface.requests.get(opcode as usize) {
        Some(desc) => desc,
        None => {
            eprintln!(
                "[wayland-backend-sys] Unknown event opcode {} for interface {}.",
                opcode, interface.name
            );
            nix::libc::abort();
        }
    };

    let mut parsed_args =
        SmallVec::<[Argument<ObjectId>; 4]>::with_capacity(message_desc.signature.len());
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
            ArgumentType::Object(_) => {
                let obj = (*args.offset(i as isize)).o as *mut wl_resource;
                if !obj.is_null() {
                    // retrieve the object relevant info
                    let obj_id = ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_resource_get_id, obj);
                    // check if this is a local or distant proxy
                    let next_interface = arg_interfaces.next().unwrap_or(&ANONYMOUS_INTERFACE);
                    let ret = ffi_dispatch!(
                        WAYLAND_SERVER_HANDLE,
                        wl_resource_instance_of,
                        obj,
                        next_interface.c_ptr.unwrap(),
                        &RUST_MANAGED as *const u8 as *const c_void
                    );
                    if ret == 0 {
                        // distant
                        parsed_args.push(Argument::Object(ObjectId {
                            alive: None,
                            id: obj_id,
                            ptr: obj,
                            interface: next_interface,
                        }));
                    } else {
                        // local
                        let obj_udata = &mut *(ffi_dispatch!(
                            WAYLAND_SERVER_HANDLE,
                            wl_resource_get_user_data,
                            obj
                        )
                            as *mut ResourceUserData<D>);
                        parsed_args.push(Argument::Object(ObjectId {
                            alive: Some(obj_udata.alive.clone()),
                            ptr: obj,
                            id: obj_id,
                            interface: obj_udata.interface,
                        }));
                    }
                } else {
                    // libwayland-server.so checks nulls for us
                    parsed_args.push(Argument::Object(ObjectId {
                        alive: None,
                        id: 0,
                        ptr: std::ptr::null_mut(),
                        interface: &ANONYMOUS_INTERFACE,
                    }))
                }
            }
            ArgumentType::NewId(_) => {
                let new_id = (*args.offset(i as isize)).n;
                // create the object
                if new_id != 0 {
                    let child_interface = match message_desc.child_interface {
                        Some(iface) => iface,
                        None => panic!("Received request {}@{}.{} which creates an object without specifying its interface, this is unsupported.", udata.interface.name, resource_id, message_desc.name),
                    };
                    let child_id = HANDLE.with(|&(handle_ptr, data_ptr)| {
                        let handle = &mut *(handle_ptr as *mut Handle<D>);
                        let data = &mut *(data_ptr as *mut D);
                        // create the object
                        let resource = ffi_dispatch!(
                            WAYLAND_SERVER_HANDLE,
                            wl_resource_create,
                            client,
                            child_interface.c_ptr.unwrap(),
                            version,
                            new_id
                        );
                        let object_udata = udata.data.clone().make_child(
                            data,
                            &ObjectInfo {
                                id: new_id,
                                interface: child_interface,
                                version: version as u32,
                            },
                        );
                        init_resource(resource, child_interface, object_udata)
                    });
                    parsed_args.push(Argument::NewId(child_id));
                } else {
                    parsed_args.push(Argument::NewId(ObjectId {
                        alive: None,
                        id: 0,
                        ptr: std::ptr::null_mut(),
                        interface: &ANONYMOUS_INTERFACE,
                    }))
                }
            }
        }
    }

    let object_id = ObjectId {
        ptr: resource,
        id: resource_id,
        interface: udata.interface,
        alive: Some(udata.alive.clone()),
    };

    let client_id = client_id_from_ptr::<D>(client).unwrap();

    HANDLE.with(|&(handle_ptr, data_ptr)| {
        let handle = &mut *(handle_ptr as *mut Handle<D>);
        let data = &mut *(data_ptr as *mut D);
        udata.data.request(
            handle,
            data,
            client_id.clone(),
            object_id.clone(),
            opcode as u16,
            &parsed_args,
        );
    });

    if message_desc.is_destructor {
        let udata = Box::from_raw(udata_ptr);
        ffi_dispatch!(
            WAYLAND_SERVER_HANDLE,
            wl_resource_set_user_data,
            resource,
            std::ptr::null_mut()
        );
        udata.alive.store(false, Ordering::Release);
        udata.data.destroyed(client_id, object_id);
        ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_resource_destroy, resource);
    }

    return 0;
}

unsafe extern "C" fn resource_destructor<D>(resource: *mut wl_resource) {
    let udata =
        Box::from_raw(ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_resource_get_user_data, resource)
            as *mut ResourceUserData<D>);
    let id = ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_resource_get_id, resource);
    let client = ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_resource_get_client, resource);
    if let Some(client_id) = client_id_from_ptr::<D>(client) {
        udata.alive.store(false, Ordering::Release);
        let object_id = ObjectId {
            interface: udata.interface,
            ptr: resource,
            alive: Some(udata.alive.clone()),
            id,
        };
        udata.data.destroyed(client_id, object_id);
    } else {
        eprintln!("[wayland-backend-sys] Destroying an object whose client is invalid... ?");
    }
}
