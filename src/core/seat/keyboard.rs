use std::slice;
use std::sync::Mutex;

use libc::c_void;

use core::From;
use core::seat::Seat;
use core::ids::{SurfaceId, wrap_surface_id};

use ffi::abi::wl_array;
use ffi::interfaces::keyboard::{wl_keyboard, wl_keyboard_destroy, wl_keyboard_release,
                                wl_keyboard_listener, wl_keyboard_add_listener};
use ffi::interfaces::surface::wl_surface;
use ffi::interfaces::seat::wl_seat_get_keyboard;
pub use ffi::enums::KeymapFormat;
pub use ffi::enums::KeyState;

use ffi::FFI;

/// An opaque unique identifier to a keyboard, can be tested for equality.
#[derive(PartialEq, Eq, Copy, Clone, Hash)]
pub struct KeyboardId {
    p: usize
}

#[inline]
pub fn wrap_keyboard_id(p: usize) -> KeyboardId {
    KeyboardId { p: p }
}

struct KeyboardData {
    format: KeymapFormat,
    keymap: Option<(i32, u32)>,
    repeat_info: Option<(i32, i32)>
}

impl KeyboardData {
    fn new() -> KeyboardData {
        KeyboardData {
            format: KeymapFormat::NoKeymap,
            keymap: None,
            repeat_info: None
        }
    }
}

struct KeyboardListener {
    keymap_handler: Box<Fn(KeymapFormat, i32, u32, &mut KeyboardData) + 'static + Send + Sync>,
    repeat_info_handler: Box<Fn(i32, i32, &mut KeyboardData) + 'static + Send + Sync>,
    enter_handler: Box<Fn(KeyboardId, SurfaceId, &[u32]) + 'static + Send + Sync>,
    leave_handler: Box<Fn(KeyboardId, SurfaceId) + 'static + Send + Sync>,
    key_handler: Box<Fn(KeyboardId, u32, u32, KeyState) + 'static + Send + Sync>,
    modifiers_handler: Box<Fn(KeyboardId, u32, u32, u32, u32) + 'static + Send + Sync>,
    data: Mutex<Box<KeyboardData>>
}

impl KeyboardListener {
    pub fn new(data: KeyboardData) -> KeyboardListener {
        KeyboardListener {
            keymap_handler: Box::new(move |kf, fd, s, data| {
                data.format = kf;
                data.keymap = Some((fd, s));
            }),
            repeat_info_handler: Box::new(move |r, d, data| {
                data.repeat_info = Some((r, d));
            }),
            enter_handler: Box::new(move |_, _, _| {}),
            leave_handler: Box::new(move |_, _| {}),
            key_handler: Box::new(move |_, _, _, _| {}),
            modifiers_handler: Box::new(move |_, _, _, _, _| {}),
            data: Mutex::new(Box::new(data))
        }
    }
}

/// A keyboard interface.
///
/// This struct is a handle to the keyboard interface of a given `Seat`.It allows
/// you to change the keyboard display and handle all keyboard-related events.
///
/// The events are handled in a callback way. Each callback are provided
/// a `KeyboardId` identifying the keyboard. This is to allow specific situation where
/// you need the keyboard to not be borrowed (to change its surface for example) and
/// have more than one cursor.
pub struct Keyboard {
    _seat: Seat,
    ptr: *mut wl_keyboard,
    listener: Box<KeyboardListener>,
}

// Keyboard is self owned
unsafe impl Send for Keyboard {}
// The wayland library guaranties this.
unsafe impl Sync for Keyboard {}

impl From<Seat> for Keyboard {
    fn from(seat: Seat) -> Keyboard {
        let ptr = unsafe { wl_seat_get_keyboard(seat.ptr_mut()) };
        let p = Keyboard {
            _seat: seat,
            ptr: ptr,
            listener: Box::new(KeyboardListener::new(KeyboardData::new()))
        };
        unsafe {
            wl_keyboard_add_listener(
                p.ptr,
                &KEYBOARD_LISTENER as *const _,
                &*p.listener as *const _ as *mut _
            );
        }
        p
    }
}

impl Keyboard {
    /// Returns the unique `KeyboardId` associated to this keyboard.
    ///
    /// This struct can be tested for equality, and will be provided in event callbacks
    /// as a mean to identify the keyboard associated with the events.
    pub fn get_id(&self) -> KeyboardId {
        wrap_keyboard_id(self.ptr as usize)
    }

    /// Provides the file descriptor giving access to the keymap definition
    /// currently used by the compositor for this keyboard.
    ///
    /// Provided data is `(fd, size)`. `size` being the number of bytes to read from `fd`.
    ///
    /// It gives away ownership of the Fd, and thus other calls of this methos will
    /// return `None`.
    ///
    /// Will also return `None` if the event providing this information has not been received
    /// yet. In such case you'll need to wait for more events to be processed. Using
    /// `Display::dispatch_pending()` for example.
    pub fn keymap_fd(&self) -> Option<(i32, u32)> {
        let mut data = self.listener.data.lock().unwrap();
        data.keymap.take()
    }

    /// Provides the repeat information of this keayboard.
    ///
    /// Provided data is `(rate, delay)`. `rate` is the rate of repeating keys, in characters
    /// per second, and `delay` is the delay in miliseconds between the moment the key is
    /// pressed and the moment it starts repeating.
    ///
    /// Will return `None` if the event providing this information has not been received
    /// yet. In such case you'll need to wait for more events to be processed. Using
    /// `Display::dispatch_pending()` for example.
    pub fn repeat_info(&self) -> Option<(i32, i32)> {
        let data = self.listener.data.lock().unwrap();
        data.repeat_info
    }

    pub fn release(self) {
        unsafe {
            wl_keyboard_release(self.ptr);
            ::std::mem::forget(self);
        }
    }

    /// Defines the action to be executed when a surface gains keyboard focus.
    ///
    /// Arguments are:
    ///
    /// - Id of the keyboard
    /// - Id of the surface
    /// - a slice of the keycodes of the currenlty pressed keys
    pub fn set_enter_action<F>(&mut self, f: F)
        where F: Fn(KeyboardId, SurfaceId, &[u32]) + 'static + Send + Sync
    {
        self.listener.enter_handler = Box::new(f)
    }

    /// Defines the action to be executed when a surface loses keyboard focus.
    ///
    /// This event is generated *before* the `enter` event is generated for the new
    /// surface the focus goes to.
    ///
    /// Arguments are:
    ///
    /// - Id of the keyboard
    /// - Id of the surface
    pub fn set_leave_action<F>(&mut self, f: F)
        where F: Fn(KeyboardId, SurfaceId) + 'static + Send + Sync
    {
        self.listener.leave_handler = Box::new(f)
    }

    /// Defines the action to be executed when a keystroke is done.
    ///
    /// Arguments are:
    ///
    /// - Id of the Keyboard
    /// - time of event (timestamp with milisecond granularity)
    /// - raw keycode of the key
    /// - new key status
    pub fn set_key_action<F>(&mut self, f: F)
        where F: Fn(KeyboardId, u32, u32, KeyState) + 'static + Send + Sync
    {
        self.listener.key_handler = Box::new(f);
    }

    /// Defines the action to be executed when a modifier is changed.
    ///
    /// This event providesthe new state of the modifiers.
    ///
    /// Arguments are:
    ///
    /// - Id of the Keyboard
    /// - mods_depressed
    /// - mods_latched
    /// - mods_locked
    /// - group
    pub fn set_modifiers_action<F>(&mut self, f: F)
        where F: Fn(KeyboardId, u32, u32, u32, u32) + 'static + Send + Sync
    {
        self.listener.modifiers_handler = Box::new(f);
    }
}

impl Drop for Keyboard {
    fn drop(&mut self) {
        unsafe { wl_keyboard_destroy(self.ptr) };
    }
}

impl FFI for Keyboard {
    type Ptr = wl_keyboard;

    fn ptr(&self) -> *const wl_keyboard {
        self.ptr as *const wl_keyboard
    }

    unsafe fn ptr_mut(&self) -> *mut wl_keyboard {
        self.ptr
    }
}

//
// C-wrappers for the callback closures, to send to wayland
//

extern "C" fn keyboard_keymap_handler(data: *mut c_void,
                                      _keyboard: *mut wl_keyboard,
                                      format: KeymapFormat,
                                      fd: i32,
                                      size: u32
                                     ) {
    let listener = unsafe { &*(data as *const KeyboardListener) };
    let mut data = listener.data.lock().unwrap();
    (*listener.keymap_handler)(format, fd, size, &mut *data);
}

extern "C" fn keyboard_enter_handler(data: *mut c_void,
                                     keyboard: *mut wl_keyboard,
                                     _serial: u32,
                                     surface: *mut wl_surface,
                                     keys: *mut wl_array,
                                   ) {
    let listener = unsafe { &*(data as *const KeyboardListener) };
    let keys = unsafe { slice::from_raw_parts((*keys).data as *const _, (*keys).size as usize) };
    (*listener.enter_handler)(
        wrap_keyboard_id(keyboard as usize),
        wrap_surface_id(surface as usize),
        keys
    );
}

extern "C" fn keyboard_leave_handler(data: *mut c_void,
                                     keyboard: *mut wl_keyboard,
                                     _serial: u32,
                                     surface: *mut wl_surface
                                    ) {
    let listener = unsafe { &*(data as *const KeyboardListener) };
    (*listener.leave_handler)(
        wrap_keyboard_id(keyboard as usize),
        wrap_surface_id(surface as usize)
    );
}

extern "C" fn keyboard_key_handler(data: *mut c_void,
                                   keyboard: *mut wl_keyboard,
                                   _serial: u32,
                                   time: u32,
                                   key: u32,
                                   state: KeyState
                                  ) {
    let listener = unsafe { &*(data as *const KeyboardListener) };
    (*listener.key_handler)(
        wrap_keyboard_id(keyboard as usize),
        time,
        key,
        state
    );
}
extern "C" fn keyboard_modifiers_handler(data: *mut c_void,
                                         keyboard: *mut wl_keyboard,
                                         _serial: u32,
                                         mods_depressed: u32,
                                         mods_latched: u32,
                                         mods_locked: u32,
                                         group: u32
                                        ) {
    let listener = unsafe { &*(data as *const KeyboardListener) };
    (*listener.modifiers_handler)(
        wrap_keyboard_id(keyboard as usize),
        mods_depressed,
        mods_latched,
        mods_locked,
        group
    );

}
extern "C" fn keyboard_repeat_info_handler(data: *mut c_void,
                                           _keyboard: *mut wl_keyboard,
                                           rate: i32,
                                           delay: i32
                                           ) {
    let listener = unsafe { &*(data as *const KeyboardListener) };
    let mut data = listener.data.lock().unwrap();
    (*listener.repeat_info_handler)(rate, delay, &mut *data);
}

static KEYBOARD_LISTENER: wl_keyboard_listener = wl_keyboard_listener {
    keymap: keyboard_keymap_handler,
    enter: keyboard_enter_handler,
    leave: keyboard_leave_handler,
    key: keyboard_key_handler,
    modifiers: keyboard_modifiers_handler,
    repeat_info: keyboard_repeat_info_handler
};
