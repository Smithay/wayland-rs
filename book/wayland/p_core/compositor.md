# The Compositor

The `wl_compositor` global is the most fundamental of the protocol: it represents the ability of the
compositor (wayland server) to display content on the screen. It probably explains why they both
have the same name (although it can be confusing at times...).

It allows the creation of surface and region objects.

## The Surfaces

The `wl_surface` represents an abstract canvas on which the client can display content. A surface
object by itself does nothing: it must first be assigned some content and a role to be displayed.

The content of a surface is assigned by sending a `wl_surface::attach` request, attaching a
`wl_buffer` to the surface (we'll learn how to create them in [next chapter](./wayland/p_core/shm.html)).
The buffer defines both the size and the pixel content of the surface. As such, a surface can be
resized simply by attaching a new buffer of a different size to it.

The role of a surface represents what it's used for. The core protocol includes 3 of them:

- Content of a window (see [The Shell](./wayland/p_core/shell.html) for details)
- Image of the pointer (see [The Seats](./wayland/p_core/seat.html) for details)
- Child surface of another surface (see [The Subcompositor](./wayland/p_core/subcompositor.html) for
  details)

But others can be introduced by other protocol extensions (for example background image,
screensaver, widget...).

A surface can only have a single role at a given time. A role is assigned to a surface by giving it
as parameter to the request creating the object representing the role.

Many of the properties of surfaces (often inherited from their roles) are double-buffered, and will
all be applied at once when the request `wl_surface::commit` is sent. Whether a property is
double-buffered is always stated in the API documentation of the requests changing it.

## The Regions

A `wl_region` represents a part of a surface. They are created as a combination of unions and
differences of rectangles.

They serve to requests the need to notify the wayland compositor about a part of a surface. For
example, the request `wl_surface::set_damage` uses a region to notify the compositor when parts of a
surface have changed and need to be redrawn.
