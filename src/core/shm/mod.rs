//! Structures related to buffers and shared memory pools.
//!
//! An `Shm` is the most classic way for a wayland client to specify
//! the contents of a `Surface` to the server.
//!
//! The `Shm` global object allows you to create `ShmPool`s out of a file descriptor,
//! which the server will `mmap` on its side to acces its contents.
//!
//! Then, you cn create `Buffer`s out of a `ShmPool`, which each sepcify a view into
//! the pool (and are allowed to overlap), and can be assigned to a `Surface`.

pub use self::buffer::Buffer;
pub use self::shm::Shm;
pub use self::shm_pool::ShmPool;

pub use self::shm::ShmFormat;

mod buffer;
mod shm;
mod shm_pool;