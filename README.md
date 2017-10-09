[![](http://meritbadge.herokuapp.com/wayland-client)](https://crates.io/crates/wayland-client)
[![Build Status](https://travis-ci.org/Smithay/wayland-rs.svg?branch=master)](https://travis-ci.org/Smithay/wayland-rs)
[![Coverage Status](https://coveralls.io/repos/github/Smithay/wayland-rs/badge.svg)](https://coveralls.io/github/Smithay/wayland-rs)

# Wayland client

These are bindings to the [reference implementation](http://wayland.freedesktop.org/)
of the wayland protocol. This is not a pure rust implementation of the wayland
protocol, and thus requires `libwayland-client.so` to be available.

This repository actually hosts 5 crates. The 3 main crates you'll likely want to use:

- *wayland-client* and *wayland-server* are the main crates for client and server side bindings
- *wayland-protocols* regroups bindings on the official protocol extentions available

And 2 internal crates, that you'll need only for integrating a custom protocol extension:

- *wayland-sys* is the actual C bindings, on which the crates are built
- *wayland-scanner* is the crate used to convert the XML protocol specifications into rust code

## Documentation

The documentation for the master branch is [available online](https://smithay.github.io/wayland-rs/).

The documentation for the releases can be found on [docs.rs](https://docs.rs/):

 - [wayland-client](https://docs.rs/wayland-client/)
 - [wayland-server](https://docs.rs/wayland-server/)
 - [wayland-protocols](https://docs.rs/wayland-protocols/)
 - [wayland-scanner](https://docs.rs/wayland-scanner/)
 - [wayland-sys](https://docs.rs/wayland-sys/)

## Requirements

Requires at least rust 1.20 to be used (using bitflags 1.0 for associated constants).
