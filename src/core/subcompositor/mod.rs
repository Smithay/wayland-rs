//! Structures related to subsurfaces and the subcompositor.
//!
//! The `SubCompositor` global object allows you to assign the `SubSurface` role
//! to a wayland `Surface`. A subsurface must have a parent surface, and will only
//! be draw if its parent is (but is does not have to be inside its parents limits).
//!
//! This allows for more complex layouts than a single-surface-window, if for example
//! not all parts of your UI need to be updated at the same time.

pub use self::subcompositor::SubCompositor;
pub use self::subsurface::SubSurface;

mod subcompositor;
mod subsurface;