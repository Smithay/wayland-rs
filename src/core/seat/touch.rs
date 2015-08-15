
use libc::c_void;

use core::From;
use core::ids::{SurfaceId, Serial, wrap_serial, wrap_surface_id};
use core::seat::Seat;

use ffi::FFI;
use ffi::abi::{wl_fixed_t, wl_fixed_to_double};
use ffi::interfaces::seat::wl_seat_get_touch;
use ffi::interfaces::touch::{wl_touch_listener, wl_touch, wl_touch_add_listener,
                             wl_touch_release, wl_touch_destroy};
use ffi::interfaces::surface::wl_surface;

/// An opaque unique identifier to a touch screen, can be tested for equality.
#[derive(PartialEq, Eq, Copy, Clone, Hash)]
pub struct TouchId {
    p: usize
}

#[inline]
pub fn wrap_touch_id(p: usize) -> TouchId {
    TouchId { p: p }
}

struct TouchListener {
    down_handler: Box<Fn(TouchId, Serial, u32, SurfaceId, i32, f64, f64) + 'static + Send + Sync>,
    up_handler: Box<Fn(TouchId, Serial, u32, i32) + 'static + Send + Sync>,
    motion_handler: Box<Fn(TouchId, u32, i32, f64, f64) + 'static + Send + Sync>,
    frame_handler: Box<Fn(TouchId) + 'static + Send + Sync>,
    cancel_handler: Box<Fn(TouchId) + 'static + Send + Sync>,
}

impl TouchListener {
    fn new() -> TouchListener {
        TouchListener {
            down_handler: Box::new(move |_,_,_,_,_,_,_| {}),
            up_handler: Box::new(move |_,_,_,_| {}),
            motion_handler: Box::new(move |_,_,_,_,_| {}),
            frame_handler: Box::new(move |_| {}),
            cancel_handler: Box::new(move |_| {})
        }
    }
}

//
// The public struct
//

pub struct Touch {
    seat: Seat,
    ptr: *mut wl_touch,
    listener: Box<TouchListener>
}

// Touch is self owned
unsafe impl Send for Touch {}
// The wayland library guaranties this.
unsafe impl Sync for Touch {}

impl From<Seat> for Touch {
    fn from(seat: Seat) -> Touch {
        let ptr = unsafe { wl_seat_get_touch(seat.ptr_mut()) };
        let p = Touch {
            seat: seat,
            ptr: ptr,
            listener: Box::new(TouchListener::new())
        };
        unsafe {
            wl_touch_add_listener(
                p.ptr,
                &TOUCH_LISTENER as *const _,
                &*p.listener as *const _ as *mut _
            );
        }
        p
    }
}

impl Touch {
    /// Returns the unique \TouchId` associated with this touch device.
    ///
    /// This struct can be tested for equality, and will be provided in event callbacks
    /// as a mean to identify the toch device associated with the events.
    pub fn get_id(&self) -> TouchId {
        wrap_touch_id(self.ptr as usize)
    }

    /// Get access to the seat associated with this keyboard
    pub fn get_seat(&self) -> &Seat {
        &self.seat
    }

    pub fn release(self) {
        unsafe {
            wl_touch_release(self.ptr);
            ::std::mem::forget(self);
        }
    }

    /// Defines the action to be executed when a new touch point appears
    ///
    /// Arguments are:
    ///
    /// - Id of the touch
    /// - serial number of the event
    /// - timestamp with miliseconds of the event
    /// - id of the surface of the event
    /// - unique id for this touch point (will be used to identify it when moving
    ///   or when disappearing)
    /// - x coordinate of the touch point on the surface
    /// - y coordinate of the touch point on the surface
    pub fn set_down_action<F>(&mut self, f: F)
        where F: Fn(TouchId, Serial, u32, SurfaceId, i32, f64, f64) + 'static + Send + Sync
    {
        self.listener.down_handler = Box::new(f)
    }

    /// Defines the action to be executed when a touch point dissapears
    ///
    /// Arguments are:
    ///
    /// - Id of the touch
    /// - serial number of the event
    /// - timestamp with miliseconds of the event
    /// - unique id for this touch point (this id is then released and can be
    ///   used for new touch points)
    pub fn set_up_action<F>(&mut self, f: F)
        where F: Fn(TouchId, Serial, u32, i32) + 'static + Send + Sync
    {
        self.listener.up_handler = Box::new(f)
    }

    /// Defines the action to be executed a touch point moves
    ///
    /// Arguments are:
    ///
    /// - Id of the touch
    /// - timestamp with miliseconds of the event
    /// - unique id for this touch point 
    /// - new x coordinate of the touch point on the surface
    /// - new y coordinate of the touch point on the surface
    pub fn set_motion_action<F>(&mut self, f: F)
        where F: Fn(TouchId, u32, i32, f64, f64) + 'static + Send + Sync
    {
        self.listener.motion_handler = Box::new(f)
    }

    pub fn set_frame_action<F>(&mut self, f: F)
        where F: Fn(TouchId) + 'static + Send + Sync
    {
        self.listener.frame_handler = Box::new(f)
    }

    /// Defines the action to be executed when a touch sequence is cancelled.
    ///
    /// This happens when the compositer recognises the sequence as a special
    /// sequence that it will interpret itself.
    ///
    /// All current touch points are cancelled and their ids released (and thus
    /// can be used for new touch points).
    pub fn set_cancel_action<F>(&mut self, f: F)
        where F: Fn(TouchId) + 'static + Send + Sync
    {
        self.listener.cancel_handler = Box::new(f)
    }
}

impl Drop for Touch {
    fn drop(&mut self) {
        unsafe { wl_touch_destroy(self.ptr) };
    }
}

impl FFI for Touch {
    type Ptr = wl_touch;

    fn ptr(&self) -> *const wl_touch {
        self.ptr as *const wl_touch
    }

    unsafe fn ptr_mut(&self) -> *mut wl_touch {
        self.ptr
    }
}


//
// C-wrappers for the callback closures, to send to wayland
//

extern "C" fn touch_down_handler(data: *mut c_void,
                                 touch: *mut wl_touch,
                                 serial: u32,
                                 time: u32,
                                 surface: *mut wl_surface,
                                 id: i32,
                                 x: wl_fixed_t,
                                 y: wl_fixed_t
                                ) {
    let listener = unsafe { &*(data as *const TouchListener) };
    (*listener.down_handler)(
        wrap_touch_id(touch as usize),
        wrap_serial(serial),
        time,
        wrap_surface_id(surface as usize),
        id,
        wl_fixed_to_double(x),
        wl_fixed_to_double(y)
    );
}

extern "C" fn touch_up_handler(data: *mut c_void,
                               touch: *mut wl_touch,
                               serial: u32,
                               time: u32,
                               id: i32
                              ) {
    let listener = unsafe { &*(data as *const TouchListener) };
    (*listener.up_handler)(
        wrap_touch_id(touch as usize),
        wrap_serial(serial),
        time,
        id
    );
}

extern "C" fn touch_motion_handler(data: *mut c_void,
                                   touch: *mut wl_touch,
                                   time: u32,
                                   id: i32,
                                   x: wl_fixed_t,
                                   y: wl_fixed_t
                                  ) {
    let listener = unsafe { &*(data as *const TouchListener) };
    (*listener.motion_handler)(
        wrap_touch_id(touch as usize),
        time,
        id,
        wl_fixed_to_double(x),
        wl_fixed_to_double(y)
    );
}

extern "C" fn touch_frame_handler(data: *mut c_void,
                                  touch: *mut wl_touch
                                 ) {
    let listener = unsafe { &*(data as *const TouchListener) };
    (*listener.frame_handler)(
        wrap_touch_id(touch as usize)
    );
}

extern "C" fn touch_cancel_handler(data: *mut c_void,
                                   touch: *mut wl_touch
                                  ) {
    let listener = unsafe { &*(data as *const TouchListener) };
    (*listener.cancel_handler)(
        wrap_touch_id(touch as usize),
    );
}

static TOUCH_LISTENER : wl_touch_listener = wl_touch_listener {
    down: touch_down_handler,
    up: touch_up_handler,
    motion : touch_motion_handler,
    frame: touch_frame_handler,
    cancel: touch_cancel_handler
};