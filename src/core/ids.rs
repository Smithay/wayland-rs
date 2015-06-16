/// An opaque unique identifier to a surface, can be tested for equality.
#[derive(PartialEq, Eq, Copy, Clone, Hash)]
pub struct SurfaceId {
    p: usize
}

#[inline]
pub fn wrap_surface_id(p: usize) -> SurfaceId {
    SurfaceId { p: p }
}

/// An opaque identifier to the serial number associated to an event.
///
/// Many events in the wayland protocol have a serial number associated to them,
/// which must be provided in queries made in response to these events.
#[derive(Copy, Clone)]
pub struct Serial {
    s: u32
}

#[inline]
pub fn wrap_serial(s: u32) -> Serial {
    Serial {
        s: s
    }
}

#[inline]
pub fn unwrap_serial(s: Serial) -> u32 {
    s.s
}