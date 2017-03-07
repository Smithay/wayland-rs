# The Shell

The `wl_shell` global allows the client to assign the role of a "window" to a surface by creating
an associated shell surface object.

> **Note:** This global is considered *deprecated* by the wayland dev team, and is apparently
> supposed to be replaced by the `xdg_shell` protocol extension.
>
> The reason for this is that this current interface is considered too simple to properly expose the
> functionality needed by popular GUI libraries (such as gtk+ or Qt), and cannot be fixed in a
> backwards-compatible way.
>
> However the `xdg_shell` extension is still in development and no stable version of it exists as of
> this writing.

## The Shell Surface

The `wl_shell_surface` is associated to a surface at creation. It adds to it various properties
making it manipulable by the compositor (and the user of the program) as what we classicaly consider
as a "window": making it able to be grabbed and moved, resized, hidden, etc...

A shell surface can be in 3 states:

- **Toplevel:** this is your classic window, taking a rectangular space in you screen.
- **Fullscreen:** takes up the whole screen. If the buffer associated to this surface does not match
  the dimensions of the screen, they'll be scaled and/or cropped by the compositor, depending on
  what the client requested.
- **Popup:** this shell surface is associated to a parent shell surface, and must be hidden or
  displayed at the same time as it.

The shell surface has a "ping" mechanism: whevener it receives a `wl_shell_surface::ping` event,
it must answer with a `wl_shell_surface::pong` request. If the answer takes too much time, the
compositor will consider this shell surface to be unresponsive and notify the user of it.

It also contains the `wl_shell_surface::configure` event, which is sent whenever the size of the
surface should change, hinting the client of the possible new size. For example when the user
started an interactive resize of the window, or a tiling window manager reorganised the windows it
displays. This new size is not an hard requirement, and the client is free to ignore it (if the
window is not resizeable) or choose a more appropriate size (for example, a terminal emulator that
can only have sizes a multiple of the size of a character).

> **Note:** The state of decorations for windows is still uncertain. Some compositors (like Gnome)
> expect clients to draw their own decorations, while others (like KDE) will be drawing server-side
> decorations. State of tiling WM (which often expect no decorations at all) is uncertain.
>
> Sadly, there is not yet a clear negotiation protocol for applications and compositors to agree
> at runtime about who will handle the decorations.
