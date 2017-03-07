# The SHM

The `wl_shm` object represents the capacity for the compositor to use SHared Memory. This is the
most basic way for a client to send the compositor the contents of its surfaces, and the only one
included in the core protocol.

The SHM global allows the client to create several memory pools. Upon creation, it receives events
for the various buffer formats supported by the compositor (such as ARGB8888 for example).

## The SHM Pools

The `wl_shm_pool` is created from the SHM global, and a file descriptor. This file descriptor is
sent to the compositor, and is the shared memory. The compositor will read from it the content
written by the client.

From an SHM pool object, the client can create buffers. Each buffer refers to a certain part of
the memory pool (specifying an offset, a width, a height and a stride).

SHM pools can be resized by the client (via the `wl_shm_pool::resize` request), but only to make
them bigger.

Creating a buffer referring to content outside of the real size of the pool is an error. The
effective behaviour of the compositor is platform-dependent: either it will read garbage (most
likely zeroed memory) and the content of the surface will silently be corrupt, or it will encounter
an error and trigger a protocol error as a consequence.

## The Buffers

The `wl_buffer` object is actually more generic than SHM pools. It can be created from SHM pools,
but also from other medium in protocol extensions (DMABUF memories for example).

In any case, it refers to some slice of memory at some place (that the compositor is supposed to
have tracked at the creation of the buffer), and can be assigned to any surface.

It possesses a single event `wl_buffer::release`, sent by the compositor when it has finished
reading from the buffer, signaling the client that the underlying content can now be modified.
