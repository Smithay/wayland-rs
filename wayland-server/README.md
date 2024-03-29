[![crates.io](https://img.shields.io/crates/v/wayland-server.svg)](https://crates.io/crates/wayland-server)
[![docs.rs](https://docs.rs/wayland-server/badge.svg)](https://docs.rs/wayland-server)
[![Continuous Integration](https://github.com/Smithay/wayland-rs/workflows/Continuous%20Integration/badge.svg)](https://github.com/Smithay/wayland-rs/actions?query=workflow%3A%22Continuous+Integration%22)
[![codecov](https://codecov.io/gh/Smithay/wayland-rs/branch/master/graph/badge.svg)](https://codecov.io/gh/Smithay/wayland-rs)

# wayland-server

Server side API for the Wayland protocol. This crate provides infrastructure for manipulating
Wayland objects, as well as object definitions for the core Wayland protocol. Protocol extensions
can be supported as well by combining this crate with `wayland-protocols`, which provides object
definitions for a large set of extensions.

**Note:** This crate is a low-level interface to the Wayland protocol. If you are looking for a more
battery-included toolkit for writing a Wayland server, you may consider
[Smithay](https://github.com/Smithay/smithay), which is a Wayland server framework built on top of it.

The crate has different backends to Wayland protocol serialization:

- By default, it uses a pure-rust implementation of the protocol, and contains little `unsafe` code.
- Activating the `use_system_lib` makes it instead bind to the system `libwayland-server.so`. This
  allows you to access C pointer versions of the wayland objects, which is necessary for interfacing
  with other non-Rust Wayland-related libraries (such as for OpenGL support, see the `wayland-egl` crate).
- Activating the `dlopen` implies `use_system_lib`, but additionaly the crate will not explicitly
  link to `libwayland-server.so` and instead try to open it at runtime, and return an error if it cannot
  find it. This allows you to build apps that can gracefully run in non-Wayland environment without needing
  compile-time switches.
