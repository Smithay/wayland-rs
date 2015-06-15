//! The core wayland protocol.
//!
//! This module covers the core part of the wayland protocol.
//!
//! The structures are organised in submodules depending on the
//! global object used to create them.

pub use self::display::{Display, default_display};
pub use self::registry::Registry;

use self::compositor::WSurface;

mod display;
mod ids;
mod registry;

pub mod compositor;
pub mod output;
pub mod seat;
pub mod shell;
pub mod shm;
pub mod subcompositor;

/// A trait for creating Wayland interfaces from each other, for
/// internal use of this library only.
trait From<T> {
    fn from(other: T) -> Self;
}

/// A trait for creating Wayland interfaces from each other in a
/// context where failure is possible, for internal use of this library only.
trait FromOpt<T> {
    fn from(other: T) -> Option<Self>;
}

/// A trait representing whatever can be used a a surface. Protocol extentions
/// surch as EGL can define their own kind of surfaces, but they wrap a `WSurface`.
pub trait Surface {
    fn get_wsurface(&self) -> &WSurface;
}

#[cfg(test)]
mod tests {
    #![allow(dead_code)]

    use super::{Buffer, Compositor, Display, Output, Pointer, Region, Registry, Seat, Shell,
                ShellSurface, Shm, ShmPool, SubCompositor, SubSurface, WSurface, Surface};

    fn require_send_sync<T: Send + Sync>() {}

    fn require_send_sync_pointer<T: Send + Sync + Surface>() {
        require_send_sync::<Pointer<T>>()
    }

    fn require_send_sync_subsurface<T: Send + Sync + Surface>() {
        require_send_sync::<SubSurface<T>>()
    }

    fn require_send_sync_shellsurface<T: Send + Sync + Surface>() {
        require_send_sync::<ShellSurface<T>>()
    }

    fn sends_syncs() {
        require_send_sync::<Buffer>();
        require_send_sync::<Compositor>();
        require_send_sync::<Display>();
        require_send_sync::<Output>();
        require_send_sync::<Region>();
        require_send_sync::<Registry>();
        require_send_sync::<Seat>();
        require_send_sync::<Shell>();
        require_send_sync::<Shm>();
        require_send_sync::<ShmPool>();
        require_send_sync::<SubCompositor>();
        require_send_sync::<WSurface>();
    }
}
