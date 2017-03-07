# The Outputs

A `wl_output` global reprensents an output the compositor has access to. Often it'll be a monitor
or a TV connected to the computer. This global only provides the client information about the
physical properties of the outputs, and can be used by other parts of the API as an identifier[^1].

The `wl_output` globals can appear and disappear as monitors are plugged into or unplugged from the
computer.

Among the information given to the client are:

- The textual name of this screen (will often be the model & manufacturer if known, or something
  like `Output #0`).
- The physical dimensions of the screen as reported by it (the unit is unspecified)
- The currently used resolution and refresh rate
- Possibly the other supported resolutions

`wl_surface`s are notified when they enter or leave an output, but cannot know where they are
located in the coordinate space of the output. The "current output" of a surface which is visible
on more than one output is the one displaying the largest fraction of it.

&nbsp;

-------

[^1]: For example, the `wl_shell::set_fullscreen` request can optionally be given an output as
argument, if the client wants its surface to be set to fullscreen on a particular output.
