use libc::c_void;

use super::{From, Seat, Surface, SurfaceId, WSurface};
use super::surface::wrap_surface_id;

use ffi::interfaces::pointer::{wl_pointer, wl_pointer_destroy, wl_pointer_set_cursor,
                               wl_pointer_release, wl_pointer_listener,
                               wl_pointer_add_listener};
use ffi::interfaces::seat::wl_seat_get_pointer;
use ffi::interfaces::surface::wl_surface;

pub use ffi::enums::wl_pointer_button_state as ButtonState;
pub use ffi::enums::wl_pointer_axis as Axis;

use ffi::FFI;

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
    enter_handler: Box<Fn(SurfaceId, i32, i32) + 'a>,
    leave_handler: Box<Fn(SurfaceId) + 'a>,
    motion_handler: Box<Fn(u32, i32, i32) + 'a>,
    button_handler: Box<Fn(u32, u32, ButtonState) + 'a>,
    axis_handler: Box<Fn(u32, Axis, i32) + 'a>
}

impl<'a> PointerListener<'a> {
    pub fn new(data: &'static CursorAdvertising) -> PointerListener<'a> {
        PointerListener {
            data: data,
            enter_handler: Box::new(move |_,_,_| {}),
            leave_handler: Box::new(move |_| {}),
            motion_handler: Box::new(move |_,_,_| {}),
            button_handler: Box::new(move |_,_,_| {}),
            axis_handler: Box::new(move |_,_,_| {}),
        }
    }
}

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

    pub fn set_cursor_hidden(&mut self, hidden: bool) {
        self.data.hidden = hidden
    }

    pub fn release(self) {
        unsafe {
            wl_pointer_release(self.ptr);
            ::std::mem::forget(self);
        }
    }

    pub fn get_surface<'c>(&'c self) -> Option<&'c S> {
        self.data.surface.as_ref()
    }

    pub fn set_enter_action<F>(&mut self, f: F)
        where F: Fn(SurfaceId, i32, i32) + 'a
    {
        self.listener.enter_handler = Box::new(f)
    }

    pub fn set_leave_action<F>(&mut self, f: F)
        where F: Fn(SurfaceId) + 'a
    {
        self.listener.leave_handler = Box::new(f)
    }

    pub fn set_motion_action<F>(&mut self, f: F)
        where F: Fn(u32, i32, i32) + 'a
    {
        self.listener.motion_handler = Box::new(f)
    }

    pub fn set_button_action<F>(&mut self, f: F)
        where F: Fn(u32, u32, ButtonState) + 'a
    {
        self.listener.button_handler = Box::new(f)
    }

    pub fn set_axis_action<F>(&mut self, f: F)
        where F: Fn(u32, Axis, i32) + 'a
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
    (*listener.enter_handler)(wrap_surface_id(surface as usize), surface_x, surface_y);
    unsafe { listener.data.advertise_cursor(serial, pointer) }
}

extern "C" fn pointer_leave_handler(data: *mut c_void,
                                    _pointer: *mut wl_pointer,
                                    _serial: u32,
                                    surface: *mut wl_surface
                                   ) {
    let listener = unsafe { &*(data as *const PointerListener) };
    (*listener.leave_handler)(wrap_surface_id(surface as usize));
}

extern "C" fn pointer_motion_handler(data: *mut c_void,
                                     _pointer: *mut wl_pointer,
                                     time: u32,
                                     surface_x: i32,
                                     surface_y: i32
                                    ) {
    let listener = unsafe { &*(data as *const PointerListener) };
    (*listener.motion_handler)(time, surface_x, surface_y);
}

extern "C" fn pointer_button_handler(data: *mut c_void,
                                     _pointer: *mut wl_pointer,
                                     _serial: u32,
                                     time: u32,
                                     button: u32,
                                     state: ButtonState
                                    ) {
    let listener = unsafe { &*(data as *const PointerListener) };
    (*listener.button_handler)(time, button, state);
}

extern "C" fn pointer_axis_handler(data: *mut c_void,
                                   _pointer: *mut wl_pointer,
                                   time: u32,
                                   axis: Axis,
                                   value: i32
                                  ) {
    let listener = unsafe { &*(data as *const PointerListener) };
    (*listener.axis_handler)(time, axis, value);
}

static POINTER_LISTENER: wl_pointer_listener = wl_pointer_listener {
    enter: pointer_enter_handler,
    leave: pointer_leave_handler,
    motion: pointer_motion_handler,
    button: pointer_button_handler,
    axis: pointer_axis_handler
};