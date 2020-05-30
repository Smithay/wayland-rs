use std::cell::RefCell;
use std::env;
use std::ffi::{OsStr, OsString};
use std::io;
use std::os::unix::io::{AsRawFd, FromRawFd, IntoRawFd, RawFd};
use std::os::unix::net::UnixListener;
use std::path::Path;
use std::rc::Rc;
use std::sync::atomic::Ordering;

use crate::display::get_runtime_dir;
use crate::{Interface, Main, Resource};

use super::clients::ClientManager;
use super::event_loop_glue::{FdManager, Token};
use super::globals::GlobalManager;
use super::{ClientInner, GlobalInner, WAYLAND_DEBUG};

pub(crate) const DISPLAY_ERROR_INVALID_OBJECT: u32 = 0;
pub(crate) const DISPLAY_ERROR_INVALID_METHOD: u32 = 1;
#[allow(dead_code)]
pub(crate) const DISPLAY_ERROR_NO_MEMORY: u32 = 2;

pub(crate) struct DisplayInner {
    epoll_mgr: Rc<FdManager>,
    pub(crate) clients_mgr: Rc<RefCell<ClientManager>>,
    global_mgr: Rc<RefCell<GlobalManager>>,
    listeners: Vec<Token>,
}

impl DisplayInner {
    pub(crate) fn new() -> DisplayInner {
        if let Some(value) = std::env::var_os("WAYLAND_DEBUG") {
            // Follow libwayland-client and enable debug log only on `1` and `server` values.
            if value == "1" || value == "server" {
                // Toggle debug log.
                WAYLAND_DEBUG.store(true, Ordering::Relaxed);
            }
        }

        let global_mgr = Rc::new(RefCell::new(GlobalManager::new()));
        let epoll_mgr = Rc::new(FdManager::new().unwrap());

        let clients_mgr =
            Rc::new(RefCell::new(ClientManager::new(epoll_mgr.clone(), global_mgr.clone())));

        DisplayInner { epoll_mgr, clients_mgr, global_mgr, listeners: Vec::new() }
    }

    pub(crate) fn create_global<I, F1, F2>(
        &mut self,
        version: u32,
        implementation: F1,
        filter: Option<F2>,
    ) -> GlobalInner<I>
    where
        I: Interface + AsRef<Resource<I>> + From<Resource<I>>,
        F1: FnMut(Main<I>, u32, crate::DispatchData<'_>) + 'static,
        F2: FnMut(ClientInner) -> bool + 'static,
    {
        self.global_mgr.borrow_mut().add_global(version, implementation, filter)
    }

    pub(crate) fn flush_clients(&mut self, data: crate::DispatchData) {
        self.clients_mgr.borrow_mut().flush_all(data)
    }

    fn add_unix_listener(&mut self, listener: UnixListener) -> io::Result<()> {
        listener.set_nonblocking(true)?;
        // The WaylandListener will automatically remove the filesystem socket
        // on drop, if any.
        let listener = WaylandListener(listener);

        let client_mgr = self.clients_mgr.clone();

        let token = self
            .epoll_mgr
            .register(listener.0.as_raw_fd(), move |mut data| {
                loop {
                    match listener.0.accept() {
                        Ok((stream, _)) => unsafe {
                            client_mgr
                                .borrow_mut()
                                .init_client(stream.into_raw_fd(), data.reborrow());
                        },
                        Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                            // we have exhausted all the pending connections
                            break;
                        }
                        Err(e) => {
                            // this is a legitimate error
                            listener.eprint_error(e);
                        }
                    }
                }
            })
            .map_err(|e| std::io::Error::from(e.as_errno().unwrap_or(nix::errno::Errno::EINVAL)))?;

        self.listeners.push(token);
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

    pub(crate) unsafe fn create_client(
        &mut self,
        fd: RawFd,
        data: crate::DispatchData,
    ) -> ClientInner {
        self.clients_mgr.borrow_mut().init_client(fd, data)
    }

    pub(crate) fn dispatch(
        &mut self,
        timeout: i32,
        data: crate::DispatchData,
    ) -> std::io::Result<()> {
        self.epoll_mgr
            .poll(timeout, data)
            .map_err(|e| From::from(e.as_errno().unwrap_or(nix::errno::Errno::EINVAL)))
    }

    pub(crate) fn get_poll_fd(&self) -> RawFd {
        self.epoll_mgr.get_poll_fd()
    }
}

impl Drop for DisplayInner {
    fn drop(&mut self) {
        for l in self.listeners.drain(..) {
            self.epoll_mgr.deregister(l);
        }
        self.clients_mgr.borrow_mut().kill_all();
    }
}

struct WaylandListener(UnixListener);

impl WaylandListener {
    fn eprint_error(&self, error: io::Error) {
        if let Ok(addr) = self.0.local_addr() {
            if let Some(path) = addr.as_pathname() {
                eprintln!(
                    "[wayland-server] Error accepting connection on listening socket {} : {}",
                    path.display(),
                    error
                );
                return;
            }
        }
        eprintln!(
            "[wayland-server] Error accepting connection on listening socket <unnamed> : {}",
            error
        );
    }
}

impl Drop for WaylandListener {
    fn drop(&mut self) {
        if let Ok(socketaddr) = self.0.local_addr() {
            if let Some(path) = socketaddr.as_pathname() {
                let _ = ::std::fs::remove_file(path);
            }
        }
    }
}
