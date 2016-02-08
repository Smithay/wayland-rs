[![Build Status](https://travis-ci.org/vberger/wayland-client-rs.svg?branch=master)](https://travis-ci.org/vberger/wayland-client-rs)
[![](http://meritbadge.herokuapp.com/wayland-client)](https://crates.io/crates/wayland-client)

# Wayland_client

These are bindings to the [reference implementation](http://wayland.freedesktop.org/)
of the wayland protocol. This is not a pure rust implementation of the wayland
protocol, and thus requires `libwayland-client.so` to be available.

The library does not actually link to the `libwayland-client.so`, but rather tries to loads it 
dynamically at first use. This allows to easily make an optionnal support for wayland on projects
using it, as wayland is not yet a largely spread technology.

## Documentation

The documentation is [available online](http://vberger.github.io/wayland-client-rs/wayland_client/).

## Wayland protocols

The `wayland-client` crate aims to allow the use of any wayland protocol extension. It'll achieve this using cargo features.

If your favourite extension is not available, feel free to open an issue to have it added. The infrastructure of the library makes it a very easy task.

### Stable extensions

Stable protocol extensions are gated by features following the naming convention `wp-<extension name>`.

Currently, no stable protocol extension exist.

### Unstable extensions

Unstable protocol extensions are both gated by a feature following the naming convention `wpu-<extension name>`
and a global cargo feature `unstable-protocols`.

The use of these protocols may require a nightly compiler, and no stability guarantee is made about any API gated
behind the `unstable-protocols` cargo feature.

Current unstable protocols available are:

- xdg-shell-unstable-v5 behind the feature `wpu-xdg_whell`
