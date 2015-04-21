//! The core wayland protocol.
//!
//! This module covers the core part of the wayland protocol.

pub use self::buffer::Buffer;
pub use self::compositor::Compositor;
pub use self::display::{Display, default_display};
pub use self::region::Region;
pub use self::registry::Registry;
pub use self::shell::Shell;
pub use self::shell_surface::ShellSurface;
pub use self::shm::Shm;
pub use self::shm_pool::ShmPool;
pub use self::subcompositor::SubCompositor;
pub use self::subsurface::SubSurface;
pub use self::surface::{Surface, WSurface};

pub use self::output::OutputTransform as OutputTransform;
pub use self::shm::ShmFormat as ShmFormat;

mod buffer;
mod compositor;
mod display;
mod output;
mod region;
mod registry;
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
