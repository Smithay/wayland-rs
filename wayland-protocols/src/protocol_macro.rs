#[macro_escape]
macro_rules! wayland_protocol(
    ($name: expr, [$(($import: ident, $interface: ident)),*]) => {
        // TODO
    }
);

#[macro_escape]
macro_rules! wayland_protocol_versioned(
    ($name: expr, [$($version: ident),*], $rest:tt) => {
        $(
            #[allow(missing_docs)]
            pub mod $version {
                wayland_protocol!(concat!($name, "-", stringify!($version)), $rest);
            }
        )*
    }
);
