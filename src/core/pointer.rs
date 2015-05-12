use libc::c_void;

use super::{From, Seat, Surface, SurfaceId, WSurface};
use super::surface::wrap_surface_id;

use ffi::interfaces::pointer::{wl_pointer, wl_pointer_destroy, wl_pointer_set_cursor,
                               wl_pointer_release, wl_pointer_listener,
                               wl_pointer_add_listener};
use ffi::interfaces::seat::wl_seat_get_pointer;
use ffi::interfaces::surface::wl_surface;
use ffi::abi::wl_fixed_to_double;

pub use ffi::enums::wl_pointer_button_state as ButtonState;
pub use ffi::enums::wl_pointer_axis as ScrollAxis;

use ffi::FFI;

/// An opaque unique identifier to a pointer, can be tested for equality.
#[derive(PartialEq, Eq, Copy, Clone, Hash)]
pub struct PointerId {
    p: usize
}

#[inline]
pub fn wrap_pointer_id(p: usize) -> PointerId {
    PointerId { p: p }
}

struct PointerData<S: Surface> {
    pub surface: Option<S>,
    pub hidden: bool,
    pub hotspot: (i32, i32)
}

impl<S: Surface> PointerData<S> {
    fn new(surface: Option<S>, hotspot: (i32, i32)) -> PointerData<S> {
        PointerData {
            surface: surface,
            hidden: false,
            hotspot: hotspot
        }
    }

    fn unwrap_surface(&mut self) -> Option<S> {
        ::std::mem::replace(&mut self.surface, None)
    }
}

trait CursorAdvertising {
    unsafe fn advertise_cursor(&self, serial: u32, pointer: *mut wl_pointer);
}

impl<S: Surface> CursorAdvertising for PointerData<S> {
        unsafe fn advertise_cursor(&self, serial: u32, pointer: *mut wl_pointer) {
        if let Some(ref s) = self.surface {
            wl_pointer_set_cursor(
                pointer,
                serial,
                s.get_wsurface().ptr_mut(),
                self.hotspot.0,
                self.hotspot.1
            )
        } else if self.hidden {
            wl_pointer_set_cursor(pointer, serial, ::std::ptr::null_mut(), 0, 0)
        }
    }
}



struct PointerListener {
    data: &'static CursorAdvertising,
    enter_handler: Box<Fn(PointerId, SurfaceId, f64, f64) + 'static + Send + Sync>,
    leave_handler: Box<Fn(PointerId, SurfaceId) + 'static + Send + Sync>,
    motion_handler: Box<Fn(PointerId, u32, f64, f64) + 'static + Send + Sync>,
    button_handler: Box<Fn(PointerId, u32, u32, ButtonState) + 'static + Send + Sync>,
    axis_handler: Box<Fn(PointerId, u32, ScrollAxis, f64) + 'static + Send + Sync>
}

impl PointerListener {
    pub fn new(data: &'static CursorAdvertising) -> PointerListener {
        PointerListener {
            data: data,
            enter_handler: Box::new(move |_,_,_,_| {}),
            leave_handler: Box::new(move |_,_| {}),
            motion_handler: Box::new(move |_,_,_,_| {}),
            button_handler: Box::new(move |_,_,_,_| {}),
            axis_handler: Box::new(move |_,_,_,_| {}),
        }
    }
}

/// A pointer interface.
///
/// This struct is a handle to the pointer interface of a given `Seat`.It allows
/// you to change the pointer display and handle all pointer-related events.
///
/// The events are handled in a callback way. Each callback are provided
/// a `PointerId` identifying the pointer. This is to allow specific situation where
/// you need the pointer to not be borrowed (to change its surface for example) and
/// have more than one cursor.
pub struct Pointer<S: Surface> {
    seat: Seat,
    ptr: *mut wl_pointer,
    listener: Box<PointerListener>,
    data: Box<PointerData<S>>
}

// Pointer is self owned
unsafe impl<S: Surface + Send> Send for Pointer<S> {}
// The wayland library guaranties this.
unsafe impl<S: Surface + Sync> Sync for Pointer<S> {}

impl From<Seat> for Pointer<WSurface> {
    fn from(seat: Seat) -> Pointer<WSurface> {
        let ptr = unsafe { wl_seat_get_pointer(seat.ptr_mut()) };
        let data = Box::new(PointerData::new(None, (0,0)));
        let p = Pointer {
            seat: seat,
            ptr: ptr,
            listener: Box::new(PointerListener::new(
                unsafe { ::std::mem::transmute(&*data as &CursorAdvertising) }
            )),
            data: data
        };
        unsafe {
            wl_pointer_add_listener(
                p.ptr,
                &POINTER_LISTENER as *const _,
                &*p.listener as *const _ as *mut _
            );
        }
        p
    }
}

impl<S: Surface> Pointer<S> {
    /// Returns the unique `PointerId` associated to this pointer.
    ///
    /// This struct can be tested for equality, and will be provided in event callbacks
    /// as a mean to identify the pointer associated with the events.
    pub fn get_id(&self) -> PointerId {
        wrap_pointer_id(self.ptr as usize)
    }

    /// Change the surface for drawing the cursor.
    ///
    /// This cursor will be changed to this new surface every time it enters
    /// a window of application. You can control this per-surface by changing
    /// the cursor surface in the `enter` event, which is processed before the
    /// surface is transmitted to the wayland server.
    ///
    /// This method consumes the `Pointer`object to create a new one and releasing 
    /// the previous surface.
    ///
    /// If surface is `None`, the cursor display won't be changed when it enters a
    /// surface of this application.
    pub fn set_cursor<NS: Surface>(mut self,
                                   surface: Option<NS>,
                                   hotspot: (i32, i32))
        -> (Pointer<NS>, Option<S>)
    {
        unsafe {
            let old_surface = self.data.unwrap_surface();
            let new_data = Box::new(PointerData::new(surface, hotspot));
            let mut listener = ::std::mem::replace(&mut self.listener, ::std::mem::uninitialized() );
            listener.data = ::std::mem::transmute(&*new_data as &CursorAdvertising) ;
            let new_pointer = Pointer {
                seat: self.seat.clone(),
                ptr: self.ptr,
                listener: listener,
                data: new_data
            };
            ::std::mem::forget(self);
            (new_pointer, old_surface)
        }
    }

    /// Hides or unhides the cursor image.
    pub fn set_cursor_hidden(&mut self, hidden: bool) {
        self.data.hidden = hidden
    }

    pub fn release(self) {
        unsafe {
            wl_pointer_release(self.ptr);
            ::std::mem::forget(self);
        }
    }

    /// Provides a reference to the current cursor surface.
    ///
    /// If no surface is set (and thus the default cursor is used), returns
    /// `None`.
    pub fn get_surface<'c>(&'c self) -> Option<&'c S> {
        self.data.surface.as_ref()
    }

    /// Defines the action to be executed when the cursor enters a surface.
    ///
    /// Arguments are:
    ///
    /// - Id of the pointer
    /// - Id of the surface
    /// - x and y, coordinates of the surface where the cursor entered
    pub fn set_enter_action<F>(&mut self, f: F)
        where F: Fn(PointerId, SurfaceId, f64, f64) + 'static + Send + Sync
    {
        self.listener.enter_handler = Box::new(f)
    }

    /// Defines the action to be executed when the cursor leaves a surface.
    ///
    /// This event is generated *before* the `enter` event is generated for the new
    /// surface the cursor goes to.
    ///
    /// Arguments are:
    ///
    /// - Id of the pointer
    /// - Id of the surface
    pub fn set_leave_action<F>(&mut self, f: F)
        where F: Fn(PointerId, SurfaceId) + 'static + Send + Sync
    {
        self.listener.leave_handler = Box::new(f)
    }

    /// Defines the action to be executed when the cursor moves
    ///
    /// Arguments are:
    ///
    /// - Id of the pointer
    /// - time of the event
    /// - x and y, new coordinates of the cursor relative to the current surface
    pub fn set_motion_action<F>(&mut self, f: F)
        where F: Fn(PointerId, u32, f64, f64) + 'static + Send + Sync
    {
        self.listener.motion_handler = Box::new(f)
    }

    /// Defines the action to be executed when a button is clicked or released
    ///
    /// Arguments are:
    ///
    /// - Id of the pointer
    /// - time of the event
    /// - button of the event,
    /// - new state of the button
    pub fn set_button_action<F>(&mut self, f: F)
        where F: Fn(PointerId, u32, u32, ButtonState) + 'static + Send + Sync
    {
        self.listener.button_handler = Box::new(f)
    }

    /// Defines the action to be executed when a scrolling is done
    ///
    /// Arguments are:
    ///
    /// - Id of the pointer
    /// - time of the event
    /// - the axis that is scrolled
    /// - the amplitude of the scrolling
    pub fn set_axis_action<F>(&mut self, f: F)
        where F: Fn(PointerId, u32, ScrollAxis, f64) + 'static + Send + Sync
    {
        self.listener.axis_handler = Box::new(f)
    }
}

impl<S: Surface> Drop for Pointer<S> {
    fn drop(&mut self) {
        unsafe { wl_pointer_destroy(self.ptr) };
    }
}

impl<S: Surface> FFI for Pointer<S> {
    type Ptr = wl_pointer;

    fn ptr(&self) -> *const wl_pointer {
        self.ptr as *const wl_pointer
    }

    unsafe fn ptr_mut(&self) -> *mut wl_pointer {
        self.ptr
    }
}

//
// C-wrappers for the callback closures, to send to wayland
//
extern "C" fn pointer_enter_handler(data: *mut c_void,
                                    pointer: *mut wl_pointer,
                                    serial: u32,
                                    surface: *mut wl_surface,
                                    surface_x: i32,
                                    surface_y: i32
                                   ) {
    let listener = unsafe { &*(data as *const PointerListener) };
    (*listener.enter_handler)(
        wrap_pointer_id(pointer as usize),
        wrap_surface_id(surface as usize),
        wl_fixed_to_double(surface_x),
        wl_fixed_to_double(surface_y)
    );
    unsafe { listener.data.advertise_cursor(serial, pointer) }
}

extern "C" fn pointer_leave_handler(data: *mut c_void,
                                    pointer: *mut wl_pointer,
                                    _serial: u32,
                                    surface: *mut wl_surface
                                   ) {
    let listener = unsafe { &*(data as *const PointerListener) };
    (*listener.leave_handler)(
        wrap_pointer_id(pointer as usize),
        wrap_surface_id(surface as usize)
    );
}

extern "C" fn pointer_motion_handler(data: *mut c_void,
                                     pointer: *mut wl_pointer,
                                     time: u32,
                                     surface_x: i32,
                                     surface_y: i32
                                    ) {
    let listener = unsafe { &*(data as *const PointerListener) };
    (*listener.motion_handler)(
        wrap_pointer_id(pointer as usize),
        time,
        wl_fixed_to_double(surface_x),
        wl_fixed_to_double(surface_y)
    );
}

extern "C" fn pointer_button_handler(data: *mut c_void,
                                     pointer: *mut wl_pointer,
                                     _serial: u32,
                                     time: u32,
                                     button: u32,
                                     state: ButtonState
                                    ) {
    let listener = unsafe { &*(data as *const PointerListener) };
    (*listener.button_handler)(
        wrap_pointer_id(pointer as usize),
        time,
        button,
        state
    );
}

extern "C" fn pointer_axis_handler(data: *mut c_void,
                                   pointer: *mut wl_pointer,
                                   time: u32,
                                   axis: ScrollAxis,
                                   value: i32
                                  ) {
    let listener = unsafe { &*(data as *const PointerListener) };
    (*listener.axis_handler)(
        wrap_pointer_id(pointer as usize),
        time,
        axis,
        wl_fixed_to_double(value)
    );
}

static POINTER_LISTENER: wl_pointer_listener = wl_pointer_listener {
    enter: pointer_enter_handler,
    leave: pointer_leave_handler,
    motion: pointer_motion_handler,
    button: pointer_button_handler,
    axis: pointer_axis_handler
};
