use std::cell::RefCell;
use std::env;
use std::ffi::{OsStr, OsString};
use std::io;
use std::os::unix::io::{FromRawFd, RawFd};
use std::os::unix::net::UnixListener;
use std::path::Path;
use std::rc::Rc;

use display::get_runtime_dir;
use {Implementation, Interface, NewResource};

use super::clients::ClientManager;
use super::event_loop::SourcesPoll;
use super::{ClientInner, EventLoopInner, GlobalInner};

pub(crate) struct DisplayInner {
    sources_poll: SourcesPoll,
    clients_mgr: Rc<RefCell<ClientManager>>,
}

impl DisplayInner {
    pub(crate) fn new() -> (Rc<RefCell<DisplayInner>>, EventLoopInner) {
        let mut evl = EventLoopInner::new();

        let display = Rc::new(RefCell::new(DisplayInner {
            sources_poll: evl.get_poll(),
            clients_mgr: Rc::new(RefCell::new(ClientManager::new(evl.get_poll()))),
        }));

        evl.display = Some(display.clone());

        (display, evl)
    }

    pub(crate) fn create_global<I: Interface, Impl>(
        &mut self,
        _: &EventLoopInner,
        version: u32,
        implementation: Impl,
    ) -> GlobalInner<I>
    where
        Impl: Implementation<NewResource<I>, u32> + 'static,
    {
        unimplemented!()
    }

    pub(crate) fn flush_clients(&mut self) {
        unimplemented!()
    }

    fn add_unix_listener(&mut self, socket: UnixListener) -> io::Result<()> {
        unimplemented!()
    }

    pub(crate) fn add_socket<S>(&mut self, name: Option<S>) -> io::Result<()>
    where
        S: AsRef<OsStr>,
    {
        // first, compute the actual socket name we will use
        let mut path = get_runtime_dir()?;

        if let Some(name) = name {
            path.push(name.as_ref());
        } else if let Some(name) = env::var_os("WAYLAND_DISPLAY") {
            let name_path: &Path = name.as_ref();
            if name_path.is_absolute() {
                path = name_path.into();
            } else {
                path.push(name_path);
            }
        } else {
            path.push("wayland-0");
        }

        let listener = UnixListener::bind(path)?;

        self.add_unix_listener(listener)
    }

    pub(crate) fn add_socket_auto(&mut self) -> io::Result<OsString> {
        let mut last_err = io::Error::new(
            io::ErrorKind::AddrInUse,
            "All sockets from wayland-0 to wayland-31 are already in use.",
        );
        for i in 0..32 {
            let name = format!("wayland-{}", i);
            match self.add_socket(Some(&name)) {
                Ok(()) => return Ok(name.into()),
                Err(e) => last_err = e,
            }
        }
        Err(last_err)
    }

    pub(crate) unsafe fn add_socket_fd(&mut self, fd: RawFd) -> io::Result<()> {
        self.add_unix_listener(FromRawFd::from_raw_fd(fd))
    }

    pub unsafe fn create_client(&mut self, fd: RawFd) -> ClientInner {
        self.clients_mgr.borrow_mut().init_client(fd)
    }
}
