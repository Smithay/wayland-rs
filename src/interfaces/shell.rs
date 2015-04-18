use super::{From, Registry, ShellSurface, Surface};

use ffi::interfaces::shell::{wl_shell, wl_shell_destroy};
use ffi::interfaces::registry::wl_registry_bind;
use ffi::{FFI, abi};

/// A handle to a wayland `wl_shell`.
///
/// This reprensent the desktop window. A surface must be bound to
/// it in order to be drawed on screen.
pub struct Shell<'a> {
    _t: ::std::marker::PhantomData<&'a ()>,
    ptr: *mut wl_shell
}

impl<'a> Shell<'a> {
    pub fn get_shell_surface<'b, 'c>(&'b self, surface: Surface<'c>) -> ShellSurface<'b, 'c> {
        From::from((self, surface))
    }
}

impl<'a, 'b> From<(&'a Registry<'b>, u32, u32)> for Shell<'a> {
    fn from((registry, id, version): (&'a Registry, u32, u32)) -> Shell<'a> {
        let ptr = unsafe { wl_registry_bind(
            registry.ptr_mut(),
            id,
            &abi::wl_shell_interface,
            version
        ) as *mut wl_shell };

        Shell {
            _t: ::std::marker::PhantomData,
            ptr: ptr
        }
    }
}

impl<'a> Drop for Shell<'a> {
    fn drop(&mut self) {
        unsafe { wl_shell_destroy(self.ptr_mut()) };
    }
}

impl<'a> FFI<wl_shell> for Shell<'a> {
    fn ptr(&self) -> *const wl_shell {
        self.ptr as *const wl_shell
    }

    unsafe fn ptr_mut(&self) -> *mut wl_shell {
        self.ptr
    }
}
