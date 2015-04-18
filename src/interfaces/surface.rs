use super::{From, Buffer, Compositor};

use ffi::interfaces::compositor::wl_compositor_create_surface;
use ffi::interfaces::surface::{wl_surface, wl_surface_destroy, wl_surface_attach,
                               wl_surface_commit};
use ffi::FFI;

pub struct Surface<'a> {
    _t: ::std::marker::PhantomData<&'a ()>,
    ptr: *mut wl_surface
}

impl<'a> Surface<'a> {
    pub fn attach(&self, buffer: &Buffer, x: i32, y: i32) {
        unsafe { wl_surface_attach(self.ptr, buffer.ptr_mut(), x, y) }
    }

    pub fn commit(&self) {
        unsafe { wl_surface_commit(self.ptr) }
    }
}

impl<'a, 'b> From<&'a Compositor<'b>> for Surface<'a> {
    fn from(compositor: &'a Compositor<'b>) -> Surface<'a> {
        let ptr = unsafe { wl_compositor_create_surface(compositor.ptr_mut()) };
        Surface {
            _t: ::std::marker::PhantomData,
            ptr: ptr
        }
    }
}

impl<'a> Drop for Surface<'a> {
    fn drop(&mut self) {
        unsafe { wl_surface_destroy(self.ptr) };
    }
}

impl<'a> FFI<wl_surface> for Surface<'a> {
    fn ptr(&self) -> *const wl_surface {
        self.ptr as *const wl_surface
    }

    unsafe fn ptr_mut(&self) -> *mut wl_surface {
        self.ptr
    }
}