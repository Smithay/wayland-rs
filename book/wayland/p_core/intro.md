# The Core protocol

As described in the previous section, the object hierarchy starts with the `wl_display`, then
the `wl_registry`, and then come all the globals from the various protocols in use.

The core wayland distribution comes with
[the core protocol specification](https://cgit.freedesktop.org/wayland/wayland/tree/protocol/wayland.xml).
It contains the `wl_display` and `wl_registry` objects[^1], as well as some globals that are*
considered the common ground that all applications will need. Any fancier functionnality is supposed
to be developped in protocol extensions.

This chapter will describe the globals (and their objects hierarchies) from this core protocol.

------

[^1] Even if `wl_display` and `wl_registry` are very special objects and kind of above all protocols,
they are still specified in the core protocol file. But their API is completely set in stone and
locked to version 1, while the rest of the core protocol can still backward-compatibly evolve.
