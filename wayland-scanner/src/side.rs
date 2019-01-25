use proc_macro2::{Ident, Span};

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
    pub(crate) fn object_name(self) -> Ident {
        Ident::new(
            match self {
                Client => "Proxy",
                Server => "Resource",
            },
            Span::call_site(),
        )
    }
}
