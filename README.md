[![crates.io](http://meritbadge.herokuapp.com/wayland-client)](https://crates.io/crates/wayland-client)
[![docs.rs](https://docs.rs/wayland-client/badge.svg)](https://docs.rs/wayland-client)
[![Continuous Integration](https://github.com/Smithay/wayland-rs/workflows/Continuous%20Integration/badge.svg)](https://github.com/Smithay/wayland-rs/actions?query=workflow%3A%22Continuous+Integration%22)
[![codecov](https://codecov.io/gh/Smithay/wayland-rs/branch/master/graph/badge.svg)](https://codecov.io/gh/Smithay/wayland-rs)

# Wayland rust crates

This project contains rust crates for using the wayland protocol, both client side and server side.

There are two ways to use them:

- By default, they use a pure rust implementation of the protocol
- If you set the `use_system_lib` cargo feature, they will rather act as bindings on top of the wayland system C
  libraries, and this will add methods to access pointers to the C objects in the API. You'll need to use this
  feature if you need to interact with a C library that requires wayland objects (typically to intialize an
  OpenGL context)

If you use the `use_system_lib` feature, the crates thus obviously require that the wayland C libs are installed
on your system. You can however require that they are dynamically loaded at startup rather than directly
linked by setting the `dlopen` flag. This can be useful if you want to ship a binary that should gracefully
handle the absence of these libs (by fallbacking to X11 for example).

This repository actually hosts 8 crates. The 3 main crates you'll likely want to use:

- *wayland-client* and *wayland-server* are the main crates for client and server side bindings
- *wayland-protocols* regroups bindings on the official protocol extentions available

There are also two auxilliary crates:

- *wayland-egl*, which is necessary client-side for OpenGL integration
- *wayland-cursor*, which helps with loading cursor images from the system themes for use in your apps

And finally 3 internal crates, that you'll need only for integrating a custom protocol extension:

- *wayland-commons* contains the protocol logic that can be shared between client-side and server-side
- *wayland-sys* is the actual C bindings, on which the crates are built
- *wayland-scanner* is the crate used to convert the XML protocol specifications into rust code

## Documentation

The documentation for the master branch is [available online](https://smithay.github.io/wayland-rs/).

The documentation for the releases can be found on [docs.rs](https://docs.rs/):

 - [wayland-client](https://docs.rs/wayland-client/)
 - [wayland-server](https://docs.rs/wayland-server/)
 - [wayland-protocols](https://docs.rs/wayland-protocols/)
 - [wayland-egl](https://docs.rs/wayland-egl/)
 - [wayland-cursor](https://docs.rs/wayland-cursor/)
 - [wayland-commons](https://docs.rs/wayland-commons/)
 - [wayland-scanner](https://docs.rs/wayland-scanner/)
 - [wayland-sys](https://docs.rs/wayland-sys/)

## Requirements

Requires at least rust 1.49 to be used, and version 1.15 of the wayland system libraries if using the
`use_system_lib` cargo feature.

## Chat and support

For general quick questions you can get answers in the chat room.

The chat room is bridged over multiple chat servers, here are 3 options on how to connect to the chat:

- [`#smithay:matrix.org`](https://matrix.to/#/#smithay:matrix.org) in Matrix
- `#smithay` on Freenode IRC
- [`Smithay/Lobby`](https://gitter.im/smithay/Lobby) on Gitter
