use wayland_sys::server::wl_display;

use globals::{GlobalResource, Global};
use requests::IteratorDispatch;

pub struct Display {
    ptr: *mut wl_display
}

impl Display {
    pub fn ptr(&self) -> *mut wl_display {
        self.ptr
    }

    pub fn declare_global<R, I>(&self, iter_disp: I) -> Global<R> where
        R: GlobalResource,
        I: IteratorDispatch + Send + Sync + 'static
    {
        ::globals::declare_global(self, iter_disp)
    }
}