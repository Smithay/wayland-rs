use super::{From, Region, Registry, WSurface};

use ffi::interfaces::compositor::{wl_compositor, wl_compositor_destroy};
use ffi::interfaces::registry::wl_registry_bind;
use ffi::{FFI, abi};

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

impl<'a, 'b> From<(&'a Registry<'b>, u32, u32)> for Compositor<'a> {
    fn from((registry, id, version): (&'a Registry, u32, u32)) -> Compositor<'a> {
        let ptr = unsafe { wl_registry_bind(
            registry.ptr_mut(),
            id,
            &abi::wl_compositor_interface,
            version
        ) as *mut wl_compositor };

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

impl<'a> FFI<wl_compositor> for Compositor<'a> {
    fn ptr(&self) -> *const wl_compositor {
        self.ptr as *const wl_compositor
    }

    unsafe fn ptr_mut(&self) -> *mut wl_compositor {
        self.ptr
    }
}