//! The core wayland protocol.
//!
//! This module covers the core part of the wayland protocol.

pub use self::buffer::Buffer;
pub use self::compositor::Compositor;
pub use self::display::{Display, default_display};
pub use self::region::Region;
pub use self::registry::Registry;
pub use self::output::{Output, OutputMode, OutputId};
pub use self::pointer::{Pointer, PointerId};
pub use self::seat::Seat;
pub use self::shell::Shell;
pub use self::shell_surface::{ShellSurface, ShellFullscreenMethod};
pub use self::shm::Shm;
pub use self::shm_pool::ShmPool;
pub use self::subcompositor::SubCompositor;
pub use self::subsurface::SubSurface;
pub use self::surface::{Surface, WSurface, SurfaceId};

pub use self::pointer::ScrollAxis as ScrollAxis;
pub use self::pointer::ButtonState as ButtonState;
pub use self::output::OutputTransform as OutputTransform;
pub use self::output::OutputSubpixel as OutputSubpixel;
pub use self::shell_surface::ShellSurfaceResize as ShellSurfaceResize;
pub use self::shm::ShmFormat as ShmFormat;

mod buffer;
mod compositor;
mod display;
mod output;
mod pointer;
mod region;
mod registry;
mod seat;
mod shell;
mod shell_surface;
mod shm;
mod shm_pool;
mod subcompositor;
mod subsurface;
mod surface;

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

#[cfg(test)]
mod tests {
    #![allow(dead_code)]

    use super::{Buffer, Compositor, Display, Output, Pointer, Region, Registry, Seat, Shell,
                Shm, ShmPool, SubSurface, WSurface, Surface};

    fn require_send_sync<T: Send + Sync>() {}

    fn require_send_sync_pointer<T: Send + Sync + Surface>() {
        require_send_sync::<Pointer<T>>()
    }

    fn require_send_sync_subsurface<T: Send + Sync + Surface>() {
        require_send_sync::<SubSurface<T>>()
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
        require_send_sync::<WSurface>();
    }
}