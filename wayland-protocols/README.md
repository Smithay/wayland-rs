[![crates.io](https://img.shields.io/crates/v/wayland-protocols.svg)](https://crates.io/crates/wayland-protocols)
[![docs.rs](https://docs.rs/wayland-protocols/badge.svg)](https://docs.rs/wayland-protocols)
[![Continuous Integration](https://github.com/Smithay/wayland-rs/workflows/Continuous%20Integration/badge.svg)](https://github.com/Smithay/wayland-rs/actions?query=workflow%3A%22Continuous+Integration%22)
[![codecov](https://codecov.io/gh/Smithay/wayland-rs/branch/master/graph/badge.svg)](https://codecov.io/gh/Smithay/wayland-rs)

# wayland-protocols

This crate provides Wayland object definitions for many of the Wayland protocol extensions available.
It is meant to be used in addition to `wayland-client` or `wayland-server`.

This crate provides bindings for the following protocols extensions:

- The standard ["wayland-protocols"](https://gitlab.freedesktop.org/wayland/wayland-protocols) extensions
- The ["wlr-protocols"](https://github.com/swaywm/wlr-protocols) extensions from wlroots
- A few other misc protocols:
  - `gtk_primary_selection`

The provided objects are controlled by cargo features:

- the `client` and `server` cargo features respectively enable the generation of client-side
  and server-side objects
- the `staging_protocols` enable the generation of protocols in the staging process and will soon become stable.
- the `unstable_protocols` enable the generation of not-yet-stabilized protocols

If you wish for other protocols to be integrated, please open an issue on Github. Only protocols that
are meant to be stabilized and largely used are in scope of this crate. If you wish to generate
bindings for your own internal protocol, you can directly use `wayland-scanner`.