use super::{From, Surface, SubSurface, WSurface};

use ffi::interfaces::subcompositor::{wl_subcompositor, wl_subcompositor_destroy};
use ffi::{FFI, Bind, abi};

/// A wayland subcompositor.
///
/// This is the back-end used to create subsurfaces.
pub struct SubCompositor<'a> {
    _t: ::std::marker::PhantomData<&'a ()>,
    ptr: *mut wl_subcompositor
}

impl<'a> SubCompositor<'a> {
    pub fn get_subsurface<'b, 'c, 'd, S>(&'b self, surface: S, parent: &'d WSurface<'d>)
        -> SubSurface<'b, 'c, 'd, S>
        where S: Surface<'c>
    {
        From::from((self, surface, parent))
    }
}

impl<'a> Drop for SubCompositor<'a> {
    fn drop(&mut self) {
        unsafe { wl_subcompositor_destroy(self.ptr) };
    }
}

impl<'a, R> Bind<'a, R> for SubCompositor<'a> {
    fn interface() -> &'static abi::wl_interface {
        abi::WAYLAND_CLIENT_HANDLE.wl_subcompositor_interface
    }

    unsafe fn wrap(ptr: *mut wl_subcompositor, _parent: &'a R) -> SubCompositor<'a> {
        SubCompositor {
            _t: ::std::marker::PhantomData,
            ptr: ptr
        }
    }
}

impl<'a> FFI for SubCompositor<'a> {
    type Ptr = wl_subcompositor;

    fn ptr(&self) -> *const wl_subcompositor {
        self.ptr as *const wl_subcompositor
    }

    unsafe fn ptr_mut(&self) -> *mut wl_subcompositor {
        self.ptr
    }
}