use self::Side::{Client, Server};

/// Side to generate
///
/// This enum represents the two possible sides of
/// the protocol API that can be generated.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Side {
    /// wayland client applications
    Client,
    /// wayland compositors
    Server,
}

impl Side {
    pub(crate) fn object_ptr_type(&self) -> &'static str {
        match *self {
            Client => "wl_proxy",
            Server => "wl_resource",
        }
    }

    pub(crate) fn object_name(&self) -> &'static str {
        match *self {
            Client => "Proxy",
            Server => "Resource",
        }
    }

    pub(crate) fn handle(&self) -> &'static str {
        match *self {
            Client => "WAYLAND_CLIENT_HANDLE",
            Server => "WAYLAND_SERVER_HANDLE",
        }
    }
}
