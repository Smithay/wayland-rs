# The Seats

A `wl_seat` global represent a logical group of input devices. Most of the time, setups will
consist of a single seat: the mouse and keyboard of the computer. However some exotic setups can
involve more than one seat[^1]. Seat globals can appear and dissapear as logical seats are created
or destroyed.

A seat can have 3 capabilities, each of them being instantiable as a different object to handle it:
keyboard, pointer, and touch. These capabilities are advertised in an event of the `wl_seat` after
its creation.

## Keyboard

A `wl_keyboard` object represents the fact that a keyboard is connected to the seat. It can be
obtained via the `wl_seat::get_keyboard` request if the seat has this capability.

This keyboard interface is very barebone: the compositor only sends the state of the modifier keys
(control, alt, shift, ...) as well as the raw keycodes of pressed and released keys. Upon creation
of the `wl_keyboard`, it will also send an event containing a file descriptor with a keymap file,
which is the one currently used by the compositor. The client is then responsible to load it (or
an other) into libxkbcommon to interpret the keystrokes as utf8 characters[^2].

The keyboard has a focus mechanism organised by surfaces. The client is notified whenever one of its
surfaces gains or loses keyboard focus, and only receives keystrokes if one of its surfaces is
currently focused.

## Pointer

The `wl_pointer` object represents the fact that a mouse or a trackpad is connected to the seat,
allowing the user to move a pointer on the screen. It can be obtained via the `wl_seat::get_pointer`
request if the seat has this capability.

The client is not allowed to track the location of the pointer over the whole screen. Instead, it
receives events whenever the pointer enters or leaves one of its surfaces, and pointer motion events
are given in the surface local coordinates: relative to its top-right corner. The client does not
receive any events if none of its surfaces are under the pointer.

A surface can also be given the role of being the pointer's image, via the `wl_pointer::set_cursor`
request. To ease this, the wayland C libraries include `libwayland-cursor`: an helper library for
loading cursor themes and getting buffers of their contents and informations about their animations.

## Touch

The `wl_touch` object represents the fact that a touchscreen is connected to the seat, providing
tactile capabilities. It can be obtained via the `wl_seat::get_touch` request if the seat has this
capability.

The events associated with the touch object allow representing multi-touch sequences, tracking the
location of each touch point. They can be cancelled if the compositor decides that it'll be the one
handling it.

&nbsp;

-------

[^1]: I don't have realistic examples to present. A simple possibility would be to plug a second
keyboard to the computer, but the compositor could decide to merge the two keyboard as a single
logical keyboard, as most computers do today. In any case, the compositor gets to decide what a
"logical seat" is.

[^2]: This is the exact function of the [wayland-kbd](https:/crates.io/crates/wayland-kbd) crate.
