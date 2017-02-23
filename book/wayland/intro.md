# The Wayland protocol

Wayland is a display protocol aiming to replace X11, and mostly developped and maintained by former
X.org developpers. It is a response to the fact that X11 is an old protocol that accumulated a lot
of unneccesary features over the years, and that its centralised architecture[^1] is far too
heavy and has performance issues.

So the choice for wayland design was to make something much more minimalistic:

 - Fuse the server, the compositor and the window manager as a single program. This means that there
   is no longer a single display server, but rather each desktop environment has its own. For
   example, both Gnome and KDE have now developped their own wayland compositor.
 - Don't provide any drawing promitive to the clients. Their are responsible of drawing the contents
   of their windows themselves, and only pass buffers to the compositor that will then blend them
   on the screen.

The built protocol aims to be much smaller and simpler than the X11 one, making actually developping
a wayland compositor a much more manageable task than building a new X11 server.

The rest of this chapter is an extensive description of this protocol and what it implies for the
clients and servers, but from a general point of view, and thus is not rust-specific at all.

---

[^1] The X11 server is at the heart of everything multiplexing between the clients, the window
manager and the compositor
