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

