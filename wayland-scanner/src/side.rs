use self::Side::{Client, Server};

/// Side to generate
///
/// This enum represents the two possible sides of
/// the protocol API that can be generated.
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Side {
    /// wayland client applications
    Client,
    /// wayland compositors
    Server,
}

#[doc(hidden)]
impl Side {
    pub fn object_ptr_type(&self) -> &'static str {
        match *self {
            Client => "wl_proxy",
            Server => "wl_resource",
        }
    }

    pub fn object_trait(&self) -> &'static str {
        match *self {
            Client => "Proxy",
            Server => "Resource",
        }
    }

    pub fn handle_type(&self) -> &'static str {
        match *self {
            Client => "EventQueueHandle",
            Server => "EventLoopHandle",
        }
    }

    pub fn handle(&self) -> &'static str {
        match *self {
            Client => "WAYLAND_CLIENT_HANDLE",
            Server => "WAYLAND_SERVER_HANDLE",
        }
    }

    pub fn result_type(&self) -> &'static str {
        match *self {
            Client => "RequestResult",
            Server => "EventResult",
        }
    }
}
