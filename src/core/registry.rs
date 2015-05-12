use std::ffi::CStr;
use std::sync::{Arc, Mutex};

use libc::{c_void, c_char, uint32_t};

use super::{From, Compositor, Display, Output, Seat, Shell, Shm, SubCompositor};

use ffi::interfaces::display::wl_display_get_registry;
use ffi::interfaces::registry::{wl_registry, wl_registry_destroy,
                                wl_registry_listener, wl_registry_add_listener,
                                wl_registry_bind};
use ffi::{FFI, Bind};

/// The internal data of the registry, wrapped into a separate
/// struct to give it easy access to the callbacks.
struct RegistryData {
    global_objects: Vec<(u32, String, u32)>,
}

impl RegistryData {
    fn new() -> RegistryData {
        RegistryData {
            global_objects: Vec::new(),
        }
    }

    fn register_object(&mut self, id: u32, interface: &[u8], version: u32) {
        self.global_objects.push(
            (id, String::from_utf8_lossy(interface).into_owned(), version)
        );
    }

    fn unregister_object(&mut self, id: u32) {
        match self.global_objects.iter().position(|e| e.0 == id) {
            Some(i) => { self.global_objects.swap_remove(i); },
            None => {}
        }
    }

    fn get_objects_with_interface(&self, interface: &str) -> Vec<(u32, u32)> {
        self.global_objects.iter().filter_map(|&(id, ref iface, v)| {
            if iface == interface {
                Some((id, v))
            } else {
                None
            }
        }).collect()
    }
}

/// The data used by the listener callbacks.
struct RegistryListener {
    /// Handler of the "new global object" event
    global_handler: Box<Fn(u32, &[u8], u32, &mut RegistryData)>,
    /// Handler of the "removed global handler" event
    global_remover: Box<Fn(u32, &mut RegistryData)>,
    /// access to the data
    data: Mutex<RegistryData>
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
            data: Mutex::new(data)
        }
    }
}

struct RegistryInternal {
    pub display: Display,
    pub ptr: *mut wl_registry,
    pub listener: Box<RegistryListener>,
}

// RegistryInternal is owning
unsafe impl Send for RegistryInternal {}

/// A Registry, giving access to the global wayland objects.
///
/// This wraps a `wl_registry`, used by the wayland to propagate
/// to the clients the global objects, such as the output or input 
/// devices, or the compositor.
///
/// This object is cloneable, allowing to create several handles to the global
/// registry. The registry will only be destroyed once all copies are destroyed.
///
/// All global objects created by the registry keep a hangle to it, and maintain
/// it alive.
#[derive(Clone)]
pub struct Registry {
    internal: Arc<Mutex<RegistryInternal>>
}

impl Registry{
    /// Returns a handle to the Display associated with this registry.
    pub fn get_display(&self) -> Display {
        self.internal.lock().unwrap().display.clone()
    }

    /// Returns a `Vec` of all global objects and their interface.
    ///
    /// Each entry is a tuple `(id, interface, version)`.
    pub fn get_global_objects(&self) -> Vec<(u32, String, u32)> {
        let internal = self.internal.lock().unwrap();
        let v = internal.listener.data.lock().unwrap().global_objects.clone();
        v
    }

    /// Returns a `Vec` of all global objects implementing given interface.
    ///
    /// Each entry is a tuple `(id, version)`.
    pub fn get_objects_with_interface(&self, interface: &str) -> Vec<(u32, u32)> {
        let internal = self.internal.lock().unwrap();
        let data = internal.listener.data.lock().unwrap();
        data.get_objects_with_interface(interface)
    }

    /// Retrieves a handle to the global compositor
    pub fn get_compositor(&self) -> Option<Compositor> {
        self.bind_with_interface("wl_compositor").into_iter().next()
    }

    /// Retrieve handles to all available outputs
    pub fn get_outputs(&self) -> Vec<Output> {
        self.bind_with_interface("wl_output")
    }

    /// Retrieve handles to all available seats
    ///
    /// A classic configuration is likely to have a single seat, but it is
    /// not garanteed.
    pub fn get_seats(&self) -> Vec<Seat> {
        self.bind_with_interface("wl_seat")
    }

    /// Retrieves a handle to the global shell
    pub fn get_shell(&self) -> Option<Shell> {
        self.bind_with_interface("wl_shell").into_iter().next()
    }

    /// Retrieves a handle to the global shm
    pub fn get_shm(&self) -> Option<Shm> {
        self.bind_with_interface("wl_shm").into_iter().next()
    }

    /// Retrieves a handle to the global subcompositor
    pub fn get_subcompositor(&self) -> Option<SubCompositor> {
        self.bind_with_interface("wl_subcompositor").into_iter().next()
    }

    pub unsafe fn bind<T>(&self, id: u32, version: u32) -> T
        where T:  Bind<Registry>
    {
        let ptr = wl_registry_bind(
            self.internal.lock().unwrap().ptr,
            id,
            <T as Bind<Registry>>::interface(),
            version
        );
        <T as Bind<Registry>>::wrap(ptr as *mut _, self.clone())
    }

    fn bind_with_interface<T>(&self, interface: &str) -> Vec<T>
        where T:  Bind<Registry>
    {
        let internal = self.internal.lock().unwrap();
        let data = internal.listener.data.lock().unwrap();
        let v = data.get_objects_with_interface(interface);
        v.into_iter().map(|(id, version)| {
            unsafe {
                let ptr = wl_registry_bind(
                    internal.ptr,
                    id,
                    <T as Bind<Registry>>::interface(),
                    version
                );
                <T as Bind<Registry>>::wrap(ptr as *mut _, self.clone())
            }
        }).collect()
    }
}

impl From<Display> for Registry {
    fn from(display: Display) -> Registry {
        let ptr = unsafe { wl_display_get_registry(display.ptr_mut()) };
        let listener = RegistryListener::default_handlers(RegistryData::new());
        let r = Registry {
            internal : Arc::new(Mutex::new(RegistryInternal {
                display: display,
                ptr: ptr,
                listener: Box::new(listener)
            }))
        };
        {
            let internal = r.internal.lock().unwrap();
            unsafe {
                wl_registry_add_listener(
                    internal.ptr,
                    &REGISTRY_LISTENER as *const _,
                    &*internal.listener as *const _ as *mut _
                )
            };
        }
        r
    }
}

impl Drop for RegistryInternal {
    fn drop(&mut self) {
        unsafe { wl_registry_destroy(self.ptr) };
    }
}

impl FFI for Registry {
    type Ptr = wl_registry;

    fn ptr(&self) -> *const wl_registry {
        self.internal.lock().unwrap().ptr as *const wl_registry
    }

    unsafe fn ptr_mut(&self) -> *mut wl_registry {
        self.internal.lock().unwrap().ptr
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
    let listener = unsafe { &*(data as *const RegistryListener) };
    let interface_str = unsafe { CStr::from_ptr(interface) };
    let mut data = listener.data.lock().unwrap();
    (listener.global_handler)(name, interface_str.to_bytes(), version, &mut *data);
}

extern "C" fn registry_global_remover(data: *mut c_void,
                                      _registry: *mut wl_registry,
                                      name: uint32_t
                                     ) {
    let listener = unsafe { &*(data as *const RegistryListener) };
    let mut data = listener.data.lock().unwrap();
    (listener.global_remover)(name, &mut *data);
}

static REGISTRY_LISTENER: wl_registry_listener = wl_registry_listener {
    global: registry_global_handler,
    global_remove: registry_global_remover
};