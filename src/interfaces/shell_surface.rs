use std::ops::Deref;

use super::{From, Shell, Surface};

use ffi::interfaces::shell::wl_shell_get_shell_surface;
use ffi::interfaces::shell_surface::{wl_shell_surface, wl_shell_surface_destroy,
                                     wl_shell_surface_set_toplevel};
use ffi::FFI;

/// A wayland `shell_surface`.
///
/// It represents a window in the most generic sense (it can be a
/// regular window, a popup, a full-screen surface, ...).
///
/// A Surface is wrapped inside this object and accessible through
/// `Deref`, so you can use a `ShellSurface` directly to update the
/// uderlying `Surface`.
pub struct ShellSurface<'a, 'b> {
    _t: ::std::marker::PhantomData<&'a ()>,
    ptr: *mut wl_shell_surface,
    surface: Surface<'b>
}

impl<'a, 'b> ShellSurface<'a, 'b> {
    /// Frees the `Surface` from its role of `shell_surface` and returns it.
    pub fn destroy(mut self) -> Surface<'b> {
        use std::mem::{forget, replace, uninitialized};
        unsafe {
            let surface = replace(&mut self.surface, uninitialized());
            wl_shell_surface_destroy(self.ptr);
            forget(self);
            surface
        }
    }

    /// Set this shell surface as being a toplevel window.
    ///
    /// It is the most classic window kind.
    pub fn set_toplevel(&self) {
        unsafe { wl_shell_surface_set_toplevel(self.ptr) }
    }
}

impl<'a, 'b> Deref for ShellSurface<'a, 'b> {
    type Target = Surface<'b>;

    fn deref<'c>(&'c self) -> &'c Surface<'b> {
        &self.surface
    }
}

impl<'a, 'b, 'c> From<(&'a Shell<'c>, Surface<'b>)> for ShellSurface<'a, 'b> {
    fn from((shell, surface): (&'a Shell<'c>, Surface<'b>)) -> ShellSurface<'a, 'b> {
        let ptr = unsafe { wl_shell_get_shell_surface(shell.ptr_mut(), surface.ptr_mut()) };
        ShellSurface {
            _t: ::std::marker::PhantomData,
            ptr: ptr,
            surface: surface
        }
    }
}

impl<'a, 'b> Drop for ShellSurface<'a, 'b> {
    fn drop(&mut self) {
        unsafe { wl_shell_surface_destroy(self.ptr) };
    }
}

impl<'a, 'b> FFI<wl_shell_surface> for ShellSurface<'a, 'b> {
    fn ptr(&self) -> *const wl_shell_surface {
        self.ptr as *const wl_shell_surface
    }

    unsafe fn ptr_mut(&self) -> *mut wl_shell_surface {
        self.ptr
    }
}