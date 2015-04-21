use super::{From, Registry, Surface, SubSurface, WSurface};

use ffi::interfaces::subcompositor::{wl_subcompositor, wl_subcompositor_destroy};
use ffi::interfaces::registry::wl_registry_bind;
use ffi::{FFI, abi};

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

impl<'a, 'b> From<(&'a Registry<'b>, u32, u32)> for SubCompositor<'a> {
    fn from((registry, id, version): (&'a Registry, u32, u32)) -> SubCompositor<'a> {
        let ptr = unsafe { wl_registry_bind(
            registry.ptr_mut(),
            id,
            &abi::wl_subcompositor_interface,
            version
        ) as *mut wl_subcompositor };

        SubCompositor {
            _t: ::std::marker::PhantomData,
            ptr: ptr
        }
    }
}

impl<'a> FFI<wl_subcompositor> for SubCompositor<'a> {
    fn ptr(&self) -> *const wl_subcompositor {
        self.ptr as *const wl_subcompositor
    }

    unsafe fn ptr_mut(&self) -> *mut wl_subcompositor {
        self.ptr
    }
}