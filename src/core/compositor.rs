use super::{From, Region, WSurface};

use ffi::interfaces::compositor::{wl_compositor, wl_compositor_destroy};
use ffi::{FFI, Bind, abi};

/// A wayland compositor.
///
/// This is the back-end that will be used for all drawing 
pub struct Compositor<'a> {
    _t: ::std::marker::PhantomData<&'a ()>,
    ptr: *mut wl_compositor
}

impl<'a> Compositor<'a> {
    /// Creates a new surface to draw on.
    pub fn create_surface<'b>(&'b self) -> WSurface<'b> {
        From::from(self)
    }

    /// Creates a new region.
    pub fn create_region<'b>(&'b self) -> Region<'b> {
        From::from(self)
    }
}

impl<'a, R> Bind<'a, R> for Compositor<'a> {
    fn interface() -> &'static abi::wl_interface {
        &abi::wl_compositor_interface
    }

    unsafe fn wrap(ptr: *mut wl_compositor, _parent: &'a R) -> Compositor<'a> {
        Compositor {
            _t: ::std::marker::PhantomData,
            ptr: ptr
        }
    }
}

impl<'a> Drop for Compositor<'a> {
    fn drop(&mut self) {
        unsafe { wl_compositor_destroy(self.ptr_mut()) };
    }
}

impl<'a> FFI for Compositor<'a> {
    type Ptr = wl_compositor;

    fn ptr(&self) -> *const wl_compositor {
        self.ptr as *const wl_compositor
    }

    unsafe fn ptr_mut(&self) -> *mut wl_compositor {
        self.ptr
    }
}