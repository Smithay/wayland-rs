use super::{From, Compositor};

use ffi::interfaces::compositor::wl_compositor_create_region;
use ffi::interfaces::region::{wl_region, wl_region_destroy};
use ffi::FFI;

pub struct Region<'a> {
    _t: ::std::marker::PhantomData<&'a ()>,
    ptr: *mut wl_region
}

impl<'a, 'b> From<&'a Compositor<'b>> for Region<'a> {
    fn from(compositor: &'a Compositor<'b>) -> Region<'a> {
        let ptr = unsafe { wl_compositor_create_region(compositor.ptr_mut()) };
        Region {
            _t: ::std::marker::PhantomData,
            ptr: ptr
        }
    }
}

impl<'a> Drop for Region<'a> {
    fn drop(&mut self) {
        unsafe { wl_region_destroy(self.ptr) };
    }
}