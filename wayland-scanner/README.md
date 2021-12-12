[![crates.io](https://img.shields.io/crates/v/wayland-scanner.svg)](https://crates.io/crates/wayland-scanner)
[![docs.rs](https://docs.rs/wayland-scanner/badge.svg)](https://docs.rs/wayland-scanner)
[![Continuous Integration](https://github.com/Smithay/wayland-rs/workflows/Continuous%20Integration/badge.svg)](https://github.com/Smithay/wayland-rs/actions?query=workflow%3A%22Continuous+Integration%22)
[![codecov](https://codecov.io/gh/Smithay/wayland-rs/branch/master/graph/badge.svg)](https://codecov.io/gh/Smithay/wayland-rs)

# wayland-scanner

Code-generation for Wayland protocols, to be used with `wayland-client` or `wayland-server`
to integrate them with your own protocol extensions.

Most general protocol extensions are already exposed by the `wayland-protocols` crate, so you
don't need to use `wayland-scanner` directly to support them.