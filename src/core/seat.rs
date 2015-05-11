use std::ffi::CStr;
use std::sync::{Arc, Mutex};

use libc::{c_void, c_char};

use super::{From, Registry, Pointer, WSurface, Keyboard};

use ffi::interfaces::seat::{wl_seat, wl_seat_destroy, wl_seat_listener, wl_seat_add_listener};
use ffi::enums::{wl_seat_capability, WL_SEAT_CAPABILITY_POINTER, WL_SEAT_CAPABILITY_KEYBOARD,
                 WL_SEAT_CAPABILITY_TOUCH};
use ffi::{FFI, Bind, abi};

struct SeatData {
    name: String,
    pointer: bool,
    keyboard: bool,
    touch: bool
}

impl SeatData {
    fn new() -> SeatData {
        SeatData {
            name: String::new(),
            pointer: false,
            keyboard: false,
            touch: false
        }
    }

    fn set_caps(&mut self, caps: wl_seat_capability) {
        self.pointer = caps.intersects(WL_SEAT_CAPABILITY_POINTER);
        self.keyboard = caps.intersects(WL_SEAT_CAPABILITY_KEYBOARD);
        self.touch = caps.intersects(WL_SEAT_CAPABILITY_TOUCH);
    }

    fn set_name(&mut self, name: String) {
        self.name = name;
    }
}

/// The data used by the listener callbacks.
struct SeatListener {
    /// Handler of the "new global object" event
    capabilities_handler: Box<Fn(wl_seat_capability, &mut SeatData)>,
    /// Handler of the "removed global handler" event
    name_handler: Box<Fn(&[u8], &mut SeatData)>,
    /// access to the data
    pub data: Mutex<SeatData>
}

impl SeatListener {
    fn default_handlers(data: SeatData) -> SeatListener {
        SeatListener {
            capabilities_handler: Box::new(move |caps, data| {
                data.set_caps(caps);
            }),
            name_handler: Box::new(move |name, data| {
                data.set_name(String::from_utf8_lossy(name).into_owned());
            }),
            data: Mutex::new(data)
        }
    }
}

struct InternalSeat {
    _registry: Registry,
    ptr: *mut wl_seat,
    listener: Box<SeatListener>
}

/// InternalSeat is self owning
unsafe impl Send for InternalSeat {}

/// A global wayland Seat.
///
/// This structure is a handle to a wayland seat, which can up to a pointer, a keyboard
/// and a touch device.
///
/// Like other global objects, this handle can be cloned.
#[derive(Clone)]
pub struct Seat {
    internal: Arc<Mutex<InternalSeat>>
}

impl Seat {
    pub fn get_pointer(&self) -> Option<Pointer<WSurface>> {
        // avoid deadlock
        let has_pointer = {
            let internal = self.internal.lock().unwrap();
            let data = internal.listener.data.lock().unwrap();
            data.pointer
        };
        if has_pointer {
            Some(From::from(self.clone()))
        } else {
            None
        }
    }

    pub fn get_keyboard(&self) -> Option<Keyboard> {
        // avoid deadlock
        let has_keyboard = {
            let internal = self.internal.lock().unwrap();
            let data = internal.listener.data.lock().unwrap();
            data.keyboard
        };
        if has_keyboard {
            Some(From::from(self.clone()))
        } else {
            None
        }
    }
}


impl Bind<Registry> for Seat {
    fn interface() -> &'static abi::wl_interface {
        abi::WAYLAND_CLIENT_HANDLE.wl_seat_interface
    }

    unsafe fn wrap(ptr: *mut wl_seat, registry: Registry) -> Seat {
        let listener_data = SeatListener::default_handlers(SeatData::new());
        let s = Seat {
            internal: Arc::new(Mutex::new(InternalSeat {
                _registry: registry,
                ptr: ptr,
                listener: Box::new(listener_data)
            }))
        };
        {
            let internal = s.internal.lock().unwrap();
            wl_seat_add_listener(
                internal.ptr,
                &SEAT_LISTENER as *const _,
                &*internal.listener as *const _ as *mut _
            );
        }
        s
    }
}

impl Drop for InternalSeat {
    fn drop(&mut self) {
        unsafe { wl_seat_destroy(self.ptr) };
    }
}

impl FFI for Seat {
    type Ptr = wl_seat;

    fn ptr(&self) -> *const wl_seat {
        self.internal.lock().unwrap().ptr as *const wl_seat
    }

    unsafe fn ptr_mut(&self) -> *mut wl_seat {
        self.internal.lock().unwrap().ptr
    }
}


//
// C-wrappers for the callback closures, to send to wayland
//
extern "C" fn seat_capabilities_handler(data: *mut c_void,
                                        _registry: *mut wl_seat,
                                        capabilities: wl_seat_capability,
                                       ) {
    let listener = unsafe { &*(data as *const SeatListener) };
    let mut data = listener.data.lock().unwrap();
    (listener.capabilities_handler)(capabilities, &mut *data);
}

extern "C" fn seat_name_handler(data: *mut c_void,
                                _registry: *mut wl_seat,
                                name: *const c_char
                               ) {
    let listener = unsafe { &*(data as *const SeatListener) };
    let name_str = unsafe { CStr::from_ptr(name) };
    let mut data = listener.data.lock().unwrap();
    (listener.name_handler)(name_str.to_bytes(), &mut *data);
}

static SEAT_LISTENER: wl_seat_listener = wl_seat_listener {
    capabilities: seat_capabilities_handler,
    name: seat_name_handler
};