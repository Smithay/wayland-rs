use libc::c_void;

use super::{From, Seat, Surface, SurfaceId, WSurface};
use super::surface::wrap_surface_id;

use ffi::interfaces::pointer::{wl_pointer, wl_pointer_destroy, wl_pointer_set_cursor,
                               wl_pointer_release, wl_pointer_listener,
                               wl_pointer_add_listener};
use ffi::interfaces::seat::wl_seat_get_pointer;
use ffi::interfaces::surface::wl_surface;

pub use ffi::enums::wl_pointer_button_state as ButtonState;
pub use ffi::enums::wl_pointer_axis as ScrollAxis;

use ffi::FFI;

/// An opaque unique identifier to a surface, can be tested for equality.
#[derive(PartialEq, Eq, Copy, Clone)]
pub struct PointerId {
    p: usize
}

#[inline]
pub fn wrap_pointer_id(p: usize) -> PointerId {
    PointerId { p: p }
}

struct PointerData<'b, S: Surface<'b>> {
    _s: ::std::marker::PhantomData<&'b ()>,
    pub surface: Option<S>,
    pub hidden: bool,
    pub hotspot: (i32, i32)
}

impl<'b, S: Surface<'b>> PointerData<'b, S> {
    fn new(surface: Option<S>, hotspot: (i32, i32)) -> PointerData<'b, S> {
        PointerData {
            _s: ::std::marker::PhantomData,
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

impl<'b, S: Surface<'b>> CursorAdvertising for PointerData<'b, S> {
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



struct PointerListener<'a> {
    data: &'static CursorAdvertising,
    enter_handler: Box<Fn(PointerId, SurfaceId, i32, i32) + 'a>,
    leave_handler: Box<Fn(PointerId, SurfaceId) + 'a>,
    motion_handler: Box<Fn(PointerId, u32, i32, i32) + 'a>,
    button_handler: Box<Fn(PointerId, u32, u32, ButtonState) + 'a>,
    axis_handler: Box<Fn(PointerId, u32, ScrollAxis, i32) + 'a>
}

impl<'a> PointerListener<'a> {
    pub fn new(data: &'static CursorAdvertising) -> PointerListener<'a> {
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
pub struct Pointer<'a, 'b, S: Surface<'b>> {
    _t: ::std::marker::PhantomData<&'a ()>,
    ptr: *mut wl_pointer,
    listener: Box<PointerListener<'a>>,
    data: Box<PointerData<'b, S>>
}

impl<'a, 'b> From<&'a Seat<'b>> for Pointer<'a, 'static, WSurface<'static>> {
    fn from(seat: &'a Seat<'b>) -> Pointer<'a, 'static, WSurface<'static>> {
        let ptr = unsafe { wl_seat_get_pointer(seat.ptr_mut()) };
        let data = Box::new(PointerData::new(None, (0,0)));
        let p = Pointer {
            _t: ::std::marker::PhantomData,
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

impl<'a, 'b, S: Surface<'b>> Pointer<'a, 'b, S> {
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
    /// This method consumes the `Pointer`object to create a new one, bound to
    /// the lifetime of the new surface and releasing the previous one.
    ///
    /// If surface is `None`, the cursor display won't be changed when it enters a
    /// surface of this application.
    pub fn set_cursor<'c, NS: Surface<'c>>(mut self,
                                           surface: Option<NS>, 
                                           hotspot: (i32, i32))
        -> (Pointer<'a, 'c, NS>, Option<S>)
    {
        unsafe {
            let old_surface = self.data.unwrap_surface();
            let new_data = Box::new(PointerData::new(surface, hotspot));
            let mut listener = ::std::mem::replace(&mut self.listener, ::std::mem::uninitialized() );
            listener.data = ::std::mem::transmute(&*new_data as &CursorAdvertising) ;
            let new_pointer = Pointer {
                _t: ::std::marker::PhantomData,
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
        where F: Fn(PointerId, SurfaceId, i32, i32) + 'a
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
        where F: Fn(PointerId, SurfaceId) + 'a
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
        where F: Fn(PointerId, u32, i32, i32) + 'a
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
        where F: Fn(PointerId, u32, u32, ButtonState) + 'a
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
        where F: Fn(PointerId, u32, ScrollAxis, i32) + 'a
    {
        self.listener.axis_handler = Box::new(f)
    }
}

impl<'a, 'b, S: Surface<'b>> Drop for Pointer<'a, 'b, S> {
    fn drop(&mut self) {
        unsafe { wl_pointer_destroy(self.ptr) };
    }
}

impl<'a, 'b, S: Surface<'b>> FFI for Pointer<'a, 'b, S> {
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
        surface_x,
        surface_y
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
        surface_x,
        surface_y
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
        value
    );
}

static POINTER_LISTENER: wl_pointer_listener = wl_pointer_listener {
    enter: pointer_enter_handler,
    leave: pointer_leave_handler,
    motion: pointer_motion_handler,
    button: pointer_button_handler,
    axis: pointer_axis_handler
};