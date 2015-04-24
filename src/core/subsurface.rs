use std::ops::Deref;

use super::{From, SubCompositor, Surface, WSurface};

use ffi::interfaces::subcompositor::wl_subcompositor_get_subsurface;
use ffi::interfaces::subsurface::{wl_subsurface, wl_subsurface_destroy,
                                  wl_subsurface_set_position,
                                  wl_subsurface_set_sync,
                                  wl_subsurface_set_desync};
use ffi::FFI;

/// A wayland subsurface.
///
/// It wraps a surface, to be integrated into a parent surface.
pub struct SubSurface<'a, 'b, 'c, S: Surface<'b>> {
    _t: ::std::marker::PhantomData<&'a ()>,
    _s: ::std::marker::PhantomData<&'b ()>,
    ptr: *mut wl_subsurface,
    surface: S,
    _parent: &'c WSurface<'c>
}

impl<'a, 'b, 'c, S: Surface<'b>> SubSurface<'a, 'b, 'c, S> {
    /// Frees the `Surface` from its role of `subsurface` and returns it.
    pub fn destroy(mut self) -> S {
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

impl<'a, 'b, 'c, S: Surface<'b>> Deref for SubSurface<'a, 'b, 'c, S> {
    type Target = S;

    fn deref<'d>(&'d self) -> &'d S {
        &self.surface
    }
}

impl<'a, 'b, 'c, 'd, S> From<(&'a SubCompositor<'d>, S, &'c WSurface<'c>)> for SubSurface<'a, 'b, 'c, S>
    where S: Surface<'b>
{
    fn from((shell, surface, parent): (&'a SubCompositor<'d>, S, &'c WSurface<'c>))
        -> SubSurface<'a, 'b, 'c, S>
    {
        let ptr = unsafe {
            wl_subcompositor_get_subsurface(
                shell.ptr_mut(),
                surface.get_wsurface().ptr_mut(),
                parent.ptr_mut()
            )
        };
        SubSurface {
            _t: ::std::marker::PhantomData,
            _s: ::std::marker::PhantomData,
            ptr: ptr,
            surface: surface,
            _parent: parent,
        }
    }
}

impl<'a, 'b, 'c, S: Surface<'b>> Drop for SubSurface<'a, 'b, 'c, S> {
    fn drop(&mut self) {
        unsafe { wl_subsurface_destroy(self.ptr) };
    }
}

impl<'a, 'b, 'c, S: Surface<'b>> FFI for SubSurface<'a, 'b, 'c, S> {
    type Ptr = wl_subsurface;

    fn ptr(&self) -> *const wl_subsurface {
        self.ptr as *const wl_subsurface
    }

    unsafe fn ptr_mut(&self) -> *mut wl_subsurface {
        self.ptr
    }
}