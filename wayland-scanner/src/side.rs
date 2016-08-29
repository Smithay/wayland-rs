use self::Side::{Server,Client};

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


