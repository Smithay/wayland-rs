use std::cell::{Cell, RefCell};
use std::ffi::CStr;

use libc::{c_void, c_char, uint32_t};

use super::{From, Compositor, Display, Shell, Shm, SubCompositor};

use ffi::interfaces::display::wl_display_get_registry;
use ffi::interfaces::registry::{wl_registry, wl_registry_destroy,
                                wl_registry_listener, wl_registry_add_listener,
                                wl_registry_bind};
use ffi::{FFI, Bind};

/// The internal data of the registry, wrapped into a separate
/// struct to give it easy access to the callbacks.
struct RegistryData {
    global_objects: RefCell<Vec<(u32, String, u32)>>,
    // the compositor is always unique
    compositor: Cell<Option<(u32, u32)>>,
    // the `wl_shell` is always unique
    shell: Cell<Option<(u32, u32)>>,
    // the `wl_shm` is always unique
    shm: Cell<Option<(u32, u32)>>,
    // the subcompositor is always unique
    subcompositor: Cell<Option<(u32, u32)>>,
}

impl RegistryData {
    fn new() -> RegistryData {
        RegistryData {
            global_objects: RefCell::new(Vec::new()),
            compositor: Cell::new(None),
            shell: Cell::new(None),
            shm: Cell::new(None),
            subcompositor: Cell::new(None),
        }
    }

    fn register_object(&self, id: u32, interface: &[u8], version: u32) {
        match interface {
            b"wl_compositor" => self.compositor.set(Some((id, version))),
            b"wl_shell" => self.shell.set(Some((id, version))),
            b"wl_shm" => self.shm.set(Some((id, version))),
            b"wl_subcompositor" => self.subcompositor.set(Some((id, version))),
            _ => {}
        }
        // register it anyway
        self.global_objects.borrow_mut().push(
            (id, String::from_utf8_lossy(interface).into_owned(), version)
        );
    }

    fn unregister_object(&self, id: u32) {
        let mut o = self.global_objects.borrow_mut();
        match o.iter().position(|e| e.0 == id) {
            Some(i) => { o.swap_remove(i); },
            None => {}
        }
    }
}

/// The data used by the listener callbacks.
struct RegistryListener {
    /// Handler of the "new global object" event
    global_handler: Box<Fn(u32, &[u8], u32, &RegistryData)>,
    /// Handler of the "removed global handler" event
    global_remover: Box<Fn(u32, &RegistryData)>,
    /// access to the data
    data: RegistryData
}

impl RegistryListener {
    fn default_handlers(data: RegistryData) -> RegistryListener {
        RegistryListener {
            global_handler: Box::new(move |id, interface, version, data| {
                data.register_object(id, interface, version);
            }),
            global_remover: Box::new(move |id, data| {
                data.unregister_object(id);
            }),
            data: data
        }
    }
}

/// A Registry, giving access to the global wayland objects.
///
/// This wraps a `wl_registry`, used by the wayland to propagate
/// to the clients the global objects, such as the output or input 
/// devices, or the compositor.
pub struct Registry<'a> {
    _m: ::std::marker::PhantomData<&'a ()>,
    ptr: *mut wl_registry,
    listener: Box<RegistryListener>,
}

macro_rules! binder {
    ($m: expr, $f: expr) => (
        match $f.get() {
            Some((id, version)) => Some(
                unsafe { $m.bind(id, version) }
            ),
            None => None
        }
    )
}

impl<'a> Registry<'a>{
    /// Returns a `Vec` of all global objects and their interface.
    ///
    /// Each entry is a tuple `(id, interface, version)`.
    pub fn get_global_objects(&self) -> Vec<(u32, String, u32)> {
        self.listener.data.global_objects.borrow().clone()
    }

    /// Returns a `Vec` of all global objects implementing given interface.
    ///
    /// Each entry is a tuple `(id, version)`.
    pub fn get_objects_with_interface(&self, interface: &str) -> Vec<(u32, u32)> {
        self.listener.data.global_objects.borrow().iter().filter_map(|&(id, ref iface, v)| {
            if iface == interface {
                Some((id, v))
            } else {
                None
            }
        }).collect()
    }

    /// Retrieves a handle to the global compositor
    pub fn get_compositor<'b>(&'b self) -> Option<Compositor<'b>> {
        binder!(self, self.listener.data.compositor)
    }

    /// Retrieves a handle to the global shell
    pub fn get_shell<'b>(&'b self) -> Option<Shell<'b>> {
        binder!(self, self.listener.data.shell)
    }

    /// Retrieves a handle to the global shm
    pub fn get_shm<'b>(&'b self) -> Option<Shm<'b>> {
        binder!(self, self.listener.data.shm)
    }

    /// Retrieves a handle to the global subcompositor
    pub fn get_subcompositor<'b>(&'b self) -> Option<SubCompositor<'b>> {
        binder!(self, self.listener.data.subcompositor)
    }

    pub unsafe fn bind<'b, T>(&'b self, id: u32, version: u32) -> T
        where T:  Bind<'b, Registry<'a>>
    {
        let ptr = wl_registry_bind(
            self.ptr,
            id,
            <T as Bind<Registry<'a>>>::interface(),
            version
        );
        <T as Bind<Registry<'a>>>::wrap(ptr as *mut _, self)
    }
}

impl<'a> From<&'a Display> for Registry<'a> {
    fn from(other: &'a Display) -> Registry<'a> {
        let ptr = unsafe { wl_display_get_registry(other.ptr_mut()) };
        let listener = RegistryListener::default_handlers(RegistryData::new());
        let r = Registry {
            _m: ::std::marker::PhantomData,
            ptr: ptr,
            listener: Box::new(listener),
        };
        unsafe {
            wl_registry_add_listener(
                r.ptr,
                &REGISTRY_LISTENER as *const _,
                &*r.listener as *const _ as *mut _
            )
        };
        r
    }
}

impl<'a> Drop for Registry<'a> {
    fn drop(&mut self) {
        unsafe { wl_registry_destroy(self.ptr_mut()) };
    }
}

impl<'a> FFI for Registry<'a> {
    type Ptr = wl_registry;

    fn ptr(&self) -> *const wl_registry {
        self.ptr as *const wl_registry
    }

    unsafe fn ptr_mut(&self) -> *mut wl_registry {
        self.ptr
    }
}

//
// C-wrappers for the callback closures, to send to wayland
//
extern "C" fn registry_global_handler(data: *mut c_void,
                                      _registry: *mut wl_registry,
                                      name: uint32_t,
                                      interface: *const c_char,
                                      version: uint32_t
                                     ) {
    let listener_data = unsafe { &*(data as *const RegistryListener) };
    let interface_str = unsafe { CStr::from_ptr(interface) };
    (listener_data.global_handler)(name, interface_str.to_bytes(), version, &listener_data.data);
}

extern "C" fn registry_global_remover(data: *mut c_void,
                                      _registry: *mut wl_registry,
                                      name: uint32_t
                                     ) {
    let listener_data = unsafe { &*(data as *const RegistryListener) };
    (listener_data.global_remover)(name, &listener_data.data);
}

static REGISTRY_LISTENER: wl_registry_listener = wl_registry_listener {
    global: registry_global_handler,
    global_remove: registry_global_remover
};