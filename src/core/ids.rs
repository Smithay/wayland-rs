/// An opaque unique identifier to a surface, can be tested for equality.
#[derive(PartialEq, Eq, Copy, Clone, Hash)]
pub struct SurfaceId {
    p: usize
}

#[inline]
pub fn wrap_surface_id(p: usize) -> SurfaceId {
    SurfaceId { p: p }
}