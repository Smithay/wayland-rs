[![crates.io](https://img.shields.io/crates/v/wayland-protocols-misc.svg)](https://crates.io/crates/wayland-protocols-misc)
[![docs.rs](https://docs.rs/wayland-protocols-misc/badge.svg)](https://docs.rs/wayland-protocols-misc)
[![Continuous Integration](https://github.com/Smithay/wayland-rs/workflows/Continuous%20Integration/badge.svg)](https://github.com/Smithay/wayland-rs/actions?query=workflow%3A%22Continuous+Integration%22)
[![codecov](https://codecov.io/gh/Smithay/wayland-rs/branch/master/graph/badge.svg)](https://codecov.io/gh/Smithay/wayland-rs)

# wayland-protocols-misc

This crate provides Wayland object definitions for various orphan or deprecated protocol extensions.
It is meant to be used in addition to `wayland-client` or `wayland-server`.

This crate provides bindings for protocols that are generally not officially supported, but are *de facto*
used by a non-negligible number of projects in the wayland ecosystem.

The provided objects are controlled by the `client` and `server` cargo features, which respectively enable
the generation of client-side and server-side objects.
