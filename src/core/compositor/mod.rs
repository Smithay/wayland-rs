//! Structures related to the compositor and surfaces.
//!
//! The central object here is the `Compositor`. It's a global Wayland object
//! provided by the `Registry`, which will allow you to create `Surface`s and
//! `Region`s.
//!
//! A `Surface` is the basic drawing bloc of a Wayland client. You can create
//! any number of them and assign them various roles, and you need to attach
//! a `Buffer` to them to define their content.
//!
//! A `Region` serves to mark a part of a `Surface`. You can see it as a "select"
//! tool of a drawing software. Various methods in this library require you to
//! provide a `Region`.

pub use self::compositor::Compositor;
pub use self::region::Region;
pub use self::surface::WSurface;
pub use core::ids::SurfaceId;

mod compositor;
mod region;
mod surface;