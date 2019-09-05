use std::cell::RefCell;
use std::env;
use std::ffi::{OsStr, OsString};
use std::io;
use std::os::unix::io::{FromRawFd, IntoRawFd, RawFd};
use std::os::unix::net::UnixListener;
use std::path::Path;
use std::rc::Rc;

use calloop::{LoopHandle, Source};

use crate::display::get_runtime_dir;
use crate::{Interface, Main, Resource};

use super::clients::ClientManager;
use super::event_loop_glue::{WSLoopHandle, WaylandListener};
use super::globals::GlobalManager;
use super::{ClientInner, GlobalInner};

pub(crate) const DISPLAY_ERROR_INVALID_OBJECT: u32 = 0;
pub(crate) const DISPLAY_ERROR_INVALID_METHOD: u32 = 1;
#[allow(dead_code)]
pub(crate) const DISPLAY_ERROR_NO_MEMORY: u32 = 2;

pub(crate) struct DisplayInner {
    loophandle: Box<dyn WSLoopHandle>,
    pub(crate) clients_mgr: Rc<RefCell<ClientManager>>,
    global_mgr: Rc<RefCell<GlobalManager>>,
    listeners: Vec<Source<WaylandListener>>,
}

impl DisplayInner {
    pub(crate) fn new<Data: 'static>(handle: LoopHandle<Data>) -> Rc<RefCell<DisplayInner>> {
        let global_mgr = Rc::new(RefCell::new(GlobalManager::new()));

        let clients_mgr = Rc::new(RefCell::new(ClientManager::new(
            Box::new(handle.clone()),
            global_mgr.clone(),
        )));

        Rc::new(RefCell::new(DisplayInner {
            loophandle: Box::new(handle.clone()),
            clients_mgr,
            global_mgr,
            listeners: Vec::new(),
        }))
    }

    pub(crate) fn create_global<I, F1, F2>(
        &mut self,
        version: u32,
        implementation: F1,
        filter: Option<F2>,
    ) -> GlobalInner<I>
    where
        I: Interface + AsRef<Resource<I>> + From<Resource<I>>,
        F1: FnMut(Main<I>, u32) + 'static,
        F2: FnMut(ClientInner) -> bool + 'static,
    {
        self.global_mgr
            .borrow_mut()
            .add_global(version, implementation, filter)
    }

    pub(crate) fn flush_clients(&mut self) {
        self.clients_mgr.borrow_mut().flush_all()
    }

    fn add_unix_listener(&mut self, listener: UnixListener) -> io::Result<()> {
        listener.set_nonblocking(true)?;

        let client_mgr = self.clients_mgr.clone();

        let source = self.loophandle.add_listener(
            WaylandListener::new(listener),
            Box::new(move |stream| unsafe {
                client_mgr.borrow_mut().init_client(stream.into_raw_fd());
            }),
        )?;

        self.listeners.push(source);
        Ok(())
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
        for i in 0..32 {
            let name = format!("wayland-{}", i);
            match self.add_socket(Some(&name)) {
                Ok(()) => return Ok(name.into()),
                Err(_) => continue,
            }
        }
        Err(io::Error::new(
            io::ErrorKind::AddrInUse,
            "All sockets from wayland-0 to wayland-31 are already in use.",
        ))
    }

    pub(crate) unsafe fn add_socket_fd(&mut self, fd: RawFd) -> io::Result<()> {
        self.add_unix_listener(FromRawFd::from_raw_fd(fd))
    }

    pub unsafe fn create_client(&mut self, fd: RawFd) -> ClientInner {
        self.clients_mgr.borrow_mut().init_client(fd)
    }
}

impl Drop for DisplayInner {
    fn drop(&mut self) {
        for l in self.listeners.drain(..) {
            l.remove();
        }
        self.clients_mgr.borrow_mut().kill_all();
    }
}
