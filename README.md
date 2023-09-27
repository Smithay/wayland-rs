[![Continuous Integration](https://github.com/Smithay/wayland-rs/workflows/Continuous%20Integration/badge.svg)](https://github.com/Smithay/wayland-rs/actions?query=workflow%3A%22Continuous+Integration%22)
[![codecov](https://codecov.io/gh/Smithay/wayland-rs/branch/master/graph/badge.svg)](https://codecov.io/gh/Smithay/wayland-rs)

# Wayland rust crates

This project contains rust crates for using the wayland protocol, both client side and server side.

This repository actually hosts 8 crates. The 3 main crates you'll likely want to use:

- **wayland-client** and **wayland-server** are the main crates for client and server side bindings
- **wayland-protocols** regroups bindings on the official protocol extentions available

There are also two auxilliary crates:

- **wayland-egl**, which is necessary client-side for OpenGL integration
- **wayland-cursor**, which helps with loading cursor images from the system themes for use in your apps

And finally 3 internal crates, that you'll need only for integrating a custom protocol extension or doing FFI:

- **wayland-scanner** is the crate used to convert the XML protocol specifications into rust code
- **wayland-backend** contains the actual implementation of the protocol logic. It actually provides two
  backends: a rust implementation of the protocol, and a backend using the system wayland libraries (for
  FFI contexts).
- **wayland-sys** is the bindings to the C wayland libraries, used by *wayland-backend*

## Documentation

The documentation for the master branch is [available online](https://smithay.github.io/wayland-rs/).

The documentation for the releases can be found on [docs.rs](https://docs.rs/):
[wayland-client](https://docs.rs/wayland-client/)
[wayland-server](https://docs.rs/wayland-server/)
[wayland-protocols](https://docs.rs/wayland-protocols/)
[wayland-egl](https://docs.rs/wayland-egl/)
[wayland-cursor](https://docs.rs/wayland-cursor/)
[wayland-backend](https://docs.rs/wayland-backend/)
[wayland-scanner](https://docs.rs/wayland-scanner/)
[wayland-sys](https://docs.rs/wayland-sys/)

## Requirements

Requires at least rust 1.65.0 to be used, and version 1.15 of the wayland system libraries if using the
system backend.

## Chat and support

You can come chat about the different wayland-rs crates, both for developpement and support, in the Matrix
chatroom [`#wayland-rs:matrix.org`](https://matrix.to/#/#wayland-rs:matrix.org).
