use self::Side::{Server,Client};

/// Side to generate
///
/// This enum represents the two possible sides of
/// the protocol API that can be generated:
///
/// - `Client` for use in wayland client applications
/// - `Server` for use in wayland compositors
#[derive(Copy,Clone,PartialEq,Eq)]
pub enum Side {
    Client,
    Server
}

#[doc(hidden)]
impl Side {
    pub fn object_ptr_type(&self) -> &'static str {
        match *self {
            Client => "wl_proxy",
            Server => "wl_resource"
        }
    }

    pub fn object_trait(&self) -> &'static str {
        match *self {
            Client => "Proxy",
            Server => "Resource"
        }
    }
}


