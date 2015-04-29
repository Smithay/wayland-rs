use super::{From, Compositor};

use ffi::interfaces::compositor::wl_compositor_create_region;
use ffi::interfaces::region::{wl_region, wl_region_destroy, wl_region_add, wl_region_subtract};
use ffi::FFI;

/// Region represent a set of pixel.
///
/// They are a way to selecta fraciton of the pixels of a surface (in
/// a similar way of the 'select' tool of a drawing software).
///
/// They are created independently of the Surface, and then attached to it.
/// (see the the documentation of Surface for methos requiring a Region)
pub struct Region {
    _compositor: Compositor,
    ptr: *mut wl_region
}

impl Region {
    /// Adds given rectangle to the region.
    ///
    /// (x, y) are he coordinate of the top-left corner.
    pub fn add(&self, x: i32, y: i32, width: i32, height: i32) {
        unsafe { wl_region_add(self.ptr, x, y, width, height) }
    }

    /// Subtract given rectangle from the region.
    ///
    /// (x, y) are he coordinate of the top-left corner.
    pub fn subtract(&self, x: i32, y: i32, width: i32, height: i32) {
        unsafe { wl_region_subtract(self.ptr, x, y, width, height) }
    }
}

impl From<Compositor> for Region {
    fn from(compositor: Compositor) -> Region {
        let ptr = unsafe { wl_compositor_create_region(compositor.ptr_mut()) };
        Region {
            _compositor: compositor,
            ptr: ptr
        }
    }
}

impl Drop for Region {
    fn drop(&mut self) {
        unsafe { wl_region_destroy(self.ptr) };
    }
}

impl FFI for Region {
    type Ptr = wl_region;

    fn ptr(&self) -> *const wl_region {
        self.ptr as *const wl_region
    }

    unsafe fn ptr_mut(&self) -> *mut wl_region {
        self.ptr
    }
}