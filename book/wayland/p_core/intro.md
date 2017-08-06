# The Core protocol

As described in the previous section, the object hierarchy starts with the `wl_display`, then
the `wl_registry`, and then come all the globals from the various protocols in use.

The core wayland distribution comes with [the core protocol specification][wayland spec]. It
contains the `wl_display` and `wl_registry` objects[^1], as well as some globals that are considered
the common ground that all applications will need. Any fancier functionality is supposed to be
developed in protocol extensions.

This chapter will describe the globals (and their objects hierarchies) from this core protocol. This
is not a complete detailed guide of all requests and events[^2], but a general description of how
they are used and interact with each other. It should be enought to properly get started with the
protocol

[wayland spec]: https://cgit.freedesktop.org/wayland/wayland/tree/protocol/wayland.xml

<br /><br />

------

[^1]: Even if `wl_display` and `wl_registry` are very special objects and kind of above all
protocols, they are still specified in the core protocol file. But their API is completely set in
stone and locked to version 1, while the rest of the core protocol can still backward-compatibly
evolve.

[^2]: Each of them is already detailed in the [API docs of wayland-client][client docs], and there's
no point copy-pasting all of it here.

[client docs]: https://smithay.github.io/wayland-rs/wayland_client/protocol/index.html
