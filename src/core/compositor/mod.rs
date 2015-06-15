//! Structures related to the compositor and surfaces.

pub use self::compositor::Compositor;
pub use self::region::Region;
pub use self::surface::WSurface;
pub use core::ids::SurfaceId;

mod compositor;
mod region;
mod surface;