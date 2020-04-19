//! Wayland cursor utilities
//!
//! This crate aims to reimplement the functionality of the `libwayland-cursor` library in Rust.
//!
//! It allows you to load cursors from the system and display them correctly.
//!
//! First of all, you need to create a `CursorTheme`,
//! which represents the full cursor theme.
//!
//! From this theme, using the `get_cursor` method, you can load a specific `Cursor`,
//! which can contain several images if the cursor is animated. It also provides you with the
//! means of querying which frame of the animation should be displayed at
//! what time, as well as handles to the buffers containing these frames, to
//! attach them to a wayland surface.

use std::{
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
    ops::{Deref, Index},
    os::unix::io::{AsRawFd, FromRawFd},
};
use wayland_client::{
    protocol::{
        wl_buffer::WlBuffer,
        wl_shm::{Format, WlShm},
        wl_shm_pool::WlShmPool,
    },
    Attached, Main,
};
use xcur::parser::File as XCurFile;
use xcursor::{theme_search_paths, XCursorTheme};

/// Represents a cursor theme loaded from the system.
pub struct CursorTheme {
    name: String,
    cursors: Vec<Cursor>,
    size: u32,
    pool: Main<WlShmPool>,
    pool_size: i32,
    file: File,
}

impl CursorTheme {
    /// Create a new cursor theme.
    pub fn new(name: &str, size: u32, shm: &Attached<WlShm>) -> Self {
        let name = String::from(name);
        let pool_size = (size * size * 4) as i32;
        let mem_fd = create_shm_fd().unwrap();
        let file = unsafe { File::from_raw_fd(mem_fd) };
        let pool = shm.create_pool(file.as_raw_fd(), pool_size);

        CursorTheme {
            name,
            file,
            size,
            pool,
            pool_size,
            cursors: Vec::new(),
        }
    }

    /// Retrieve a cursor from the theme.
    ///
    /// This method returns `None` if this cursor is not provided
    /// either by the theme, or by one of its parents.
    pub fn get_cursor(&mut self, name: &str) -> Option<&Cursor> {
        let cur = self.cursors.iter().position(|i| i.name == name);

        match cur {
            Some(i) => Some(&self.cursors[i]),
            None => {
                let cur = self.load_cursor(name, self.size).unwrap();
                self.cursors.push(cur);
                self.cursors.iter().last()
            }
        }
    }

    /// This function loads a cursor, parses it and
    /// pushes the images onto the shm pool.
    /// Keep in mind that if the cursor is already loaded,
    /// the function will make a duplicate.
    fn load_cursor(&mut self, name: &str, size: u32) -> Option<Cursor> {
        let icon_path = XCursorTheme::load(&self.name, &theme_search_paths()).load_icon(name);
        let mut icon_file = File::open(icon_path.unwrap()).ok()?;

        let mut buf = Vec::new();
        let xcur = {
            icon_file.read_to_end(&mut buf).ok()?;
            XCurFile::parse(&buf)
        };

        // Terminate if cursor can't be parsed
        if !xcur.is_done() {
            return None;
        }

        let file_images = xcur.unwrap().1.images;
        let cursor = Cursor::new(name, self, &file_images, size);

        Some(cursor)
    }

    /// Grow the wl_shm_pool this theme is stored on.
    /// This method does nothing if the provided size is
    /// smaller or equal to the pool's current size.
    fn grow(&mut self, size: i32) {
        if size > self.pool_size {
            self.pool.resize(size);
            self.pool_size = size;
        }
    }
}

/// A cursor from a theme. Can contain several images if animated.
#[derive(Clone)]
pub struct Cursor {
    name: String,
    images: Vec<CursorImageBuffer>,
}

impl Cursor {
    /// Construct a new Cursor.
    ///
    /// Each of the provided images will be written into `theme`.
    /// This will also grow `theme.pool` if necessary.
    fn new(name: &str, theme: &mut CursorTheme, images: &[xcur::parser::Image], size: u32) -> Self {
        let mut buffers = Vec::with_capacity(images.len());
        let iter = images.iter().filter(|el| {el.width == size && el.height == size});

        for img in iter {
            buffers.push(CursorImageBuffer::new(theme, img));
        }

        Cursor {
            name: String::from(name),
            images: buffers,
        }
    }

    /// Given a time, calculate which frame to show, and how much time remains until the next frame.
    ///
    /// Time will wrap, so if for instance the cursor has an animation during 100ms,
    /// then calling this function with 5ms and 105ms as input gives the same output.
    pub fn frame_and_duration(&self, mut millis: u32) -> FrameAndDuration {
        let mut iter = self.images.iter().enumerate().cycle();
        loop {
            let (i, img) = iter.next().unwrap();
            if millis > img.delay {
                millis -= img.delay;
            } else {
                return FrameAndDuration {
                    frame_index: i,
                    frame_duration: millis,
                };
            }
        }
    }
}

impl Index<usize> for Cursor {
    type Output = CursorImageBuffer;

    fn index(&self, index: usize) -> &Self::Output {
        &self.images[index]
    }
}

/// A buffer containing a cursor image.
///
/// You can access the `WlBuffer` via `Deref`.
///
/// Note that this proxy will be considered as "unmanaged" by the crate, as such you should
/// not try to act on it beyond assigning it to `wl_surface`s.
#[derive(Clone)]
pub struct CursorImageBuffer {
    buffer: WlBuffer,
    delay: u32,
    xhot: u32,
    yhot: u32,
    width: u32,
    height: u32,
}

impl CursorImageBuffer {
    /// Construct a new CursorImageBuffer
    ///
    /// This function appends the pixels of the image to the provided file,
    /// and constructs a wl_buffer on that data.
    fn new(theme: &mut CursorTheme, image: &xcur::parser::Image) -> Self {
        let buf = CursorImageBuffer::convert_pixels(&image.pixels);
        let offset = theme.file.seek(SeekFrom::End(0)).unwrap();
        theme.file.write_all(&buf).unwrap();

        let new_size = theme.file.seek(SeekFrom::End(0)).unwrap();
        theme.grow(new_size as i32);

        let buffer = theme
            .pool
            .create_buffer(
                offset as i32,
                image.width as i32,
                image.height as i32,
                (image.width * 4) as i32,
                Format::Argb8888,
            )
            .detach();

        CursorImageBuffer {
            buffer,
            delay: image.delay,
            xhot: image.xhot,
            yhot: image.yhot,
            width: image.width,
            height: image.height,
        }
    }

    /// Convert the pixels saved in `u32`s into `u8`s.
    fn convert_pixels(pixels: &[u32]) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::with_capacity(pixels.len() * 4);

        for pixel in pixels {
            buf.push((pixel >> 24) as u8);
            buf.push((pixel >> 16) as u8);
            buf.push((pixel >> 8) as u8);
            buf.push(*pixel as u8);
        }

        buf
    }
}

impl Deref for CursorImageBuffer {
    type Target = WlBuffer;
    fn deref(&self) -> &WlBuffer {
        &self.buffer
    }
}

/// Which frame to show, and for how long.
///
/// This struct is output by `Cursor::frame_and_duration`
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FrameAndDuration {
    /// The index of the frame which should be shown.
    pub frame_index: usize,
    /// The duration that the frame should be shown for.
    pub frame_duration: u32,
}

/// Create a shared file descriptor in memory
use {
    nix::{
        errno::Errno,
        fcntl,
        sys::{memfd, mman, stat},
        unistd,
    },
    std::{
        ffi::CStr,
        io,
        os::unix::io::RawFd,
        time::{SystemTime, UNIX_EPOCH},
    },
};
fn create_shm_fd() -> io::Result<RawFd> {
    // Only try memfd on linux
    #[cfg(target_os = "linux")]
    loop {
        match memfd::memfd_create(
            CStr::from_bytes_with_nul(b"smithay-client-toolkit\0").unwrap(),
            memfd::MemFdCreateFlag::MFD_CLOEXEC,
        ) {
            Ok(fd) => return Ok(fd),
            Err(nix::Error::Sys(Errno::EINTR)) => continue,
            Err(nix::Error::Sys(Errno::ENOSYS)) => break,
            Err(nix::Error::Sys(errno)) => return Err(io::Error::from(errno)),
            Err(err) => unreachable!(err),
        }
    }

    // Fallback to using shm_open
    let sys_time = SystemTime::now();
    let mut mem_file_handle = format!(
        "/smithay-client-toolkit-{}",
        sys_time.duration_since(UNIX_EPOCH).unwrap().subsec_nanos()
    );
    loop {
        match mman::shm_open(
            mem_file_handle.as_str(),
            fcntl::OFlag::O_CREAT | fcntl::OFlag::O_EXCL | fcntl::OFlag::O_RDWR | fcntl::OFlag::O_CLOEXEC,
            stat::Mode::S_IRUSR | stat::Mode::S_IWUSR,
        ) {
            Ok(fd) => match mman::shm_unlink(mem_file_handle.as_str()) {
                Ok(_) => return Ok(fd),
                Err(nix::Error::Sys(errno)) => match unistd::close(fd) {
                    Ok(_) => return Err(io::Error::from(errno)),
                    Err(nix::Error::Sys(errno)) => return Err(io::Error::from(errno)),
                    Err(err) => panic!(err),
                },
                Err(err) => panic!(err),
            },
            Err(nix::Error::Sys(Errno::EEXIST)) => {
                // If a file with that handle exists then change the handle
                mem_file_handle = format!(
                    "/smithay-client-toolkit-{}",
                    sys_time.duration_since(UNIX_EPOCH).unwrap().subsec_nanos()
                );
                continue;
            }
            Err(nix::Error::Sys(Errno::EINTR)) => continue,
            Err(nix::Error::Sys(errno)) => return Err(io::Error::from(errno)),
            Err(err) => unreachable!(err),
        }
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_convert_pixels() {
        let pixels: &[u32] = &[0x12345678, 0x87654321];
        let parsed_pixels: &[u8] = &[0x12, 0x34, 0x56, 0x78, 0x87, 0x65, 0x43, 0x21];

        assert_eq!(
            super::CursorImageBuffer::convert_pixels(&pixels),
            Vec::from(parsed_pixels)
        );
    }
}
