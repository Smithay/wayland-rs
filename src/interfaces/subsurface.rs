use std::ops::Deref;

use super::{From, SubCompositor, Surface};

use ffi::interfaces::subcompositor::wl_subcompositor_get_subsurface;
use ffi::interfaces::subsurface::{wl_subsurface, wl_subsurface_destroy,
                                  wl_subsurface_set_position,
                                  wl_subsurface_set_sync,
                                  wl_subsurface_set_desync};
use ffi::FFI;

/// A wayland subsurface.
///
/// It wraps a surface, to be integrated into a parent surface.
pub struct SubSurface<'a, 'b, 'c> {
    _t: ::std::marker::PhantomData<&'a ()>,
    ptr: *mut wl_subsurface,
    surface: Surface<'b>,
    _parent: &'c Surface<'c>
}

impl<'a, 'b, 'c> SubSurface<'a, 'b, 'c> {
    /// Frees the `Surface` from its role of `subsurface` and returns it.
    pub fn destroy(mut self) -> Surface<'b> {
        use std::mem::{forget, replace, uninitialized};
        unsafe {
            let surface = replace(&mut self.surface, uninitialized());
            wl_subsurface_destroy(self.ptr);
            forget(self);
            surface
        }
    }

    /// Sets the position of the subsurface in the parent surface coordinate
    /// system.
    ///
    /// The change will tale effect at the next `Surface::commit()` call on the parent
    /// surface.
    pub fn set_position(&self, x: i32, y: i32) {
        unsafe { wl_subsurface_set_position(self.ptr, x, y) }
    }

    /// Sets of unsets the subsurface into synchronysed mode.
    ///
    /// When in synchronised mode, the changes of this surface are not applied when
    /// calling `commit()`, but rather cached, and will be applied when the parents
    /// commit is called.
    ///
    /// When a subsurface is set to synchronised mode, all its children subsurfaces
    /// are forcedinto this mode as well.
    pub fn set_sync(&self, b: bool) {
        if b {
            unsafe { wl_subsurface_set_sync(self.ptr) }
        } else {
            unsafe { wl_subsurface_set_desync(self.ptr) }
        }
    }

}

impl<'a, 'b, 'c> Deref for SubSurface<'a, 'b, 'c> {
    type Target = Surface<'b>;

    fn deref<'d>(&'d self) -> &'d Surface<'b> {
        &self.surface
    }
}

impl<'a, 'b, 'c, 'd> From<(&'a SubCompositor<'d>, Surface<'b>, &'c Surface<'c>)> for SubSurface<'a, 'b, 'c> {
    fn from((shell, surface, parent): (&'a SubCompositor<'d>, Surface<'b>, &'c Surface<'c>)) -> SubSurface<'a, 'b, 'c> {
        let ptr = unsafe { wl_subcompositor_get_subsurface(shell.ptr_mut(), surface.ptr_mut(), parent.ptr_mut()) };
        SubSurface {
            _t: ::std::marker::PhantomData,
            ptr: ptr,
            surface: surface,
            _parent: parent,
        }
    }
}

impl<'a, 'b, 'c> Drop for SubSurface<'a, 'b, 'c> {
    fn drop(&mut self) {
        unsafe { wl_subsurface_destroy(self.ptr) };
    }
}

impl<'a, 'b, 'c> FFI<wl_subsurface> for SubSurface<'a, 'b, 'c> {
    fn ptr(&self) -> *const wl_subsurface {
        self.ptr as *const wl_subsurface
    }

    unsafe fn ptr_mut(&self) -> *mut wl_subsurface {
        self.ptr
    }
}