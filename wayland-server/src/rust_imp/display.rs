use std::cell::RefCell;
use std::env;
use std::ffi::{OsStr, OsString};
use std::io;
use std::os::unix::io::{AsRawFd, FromRawFd, IntoRawFd, RawFd};
use std::os::unix::net::UnixListener;
use std::path::Path;
use std::rc::Rc;

use mio::Ready;

use display::get_runtime_dir;
use sources::{FdEvent, FdInterest};
use {Interface, NewResource};

use super::clients::ClientManager;
use super::event_loop::SourcesPoll;
use super::globals::GlobalManager;
use super::{ClientInner, EventLoopInner, GlobalInner, SourceInner};

pub(crate) const DISPLAY_ERROR_INVALID_OBJECT: u32 = 0;
pub(crate) const DISPLAY_ERROR_INVALID_METHOD: u32 = 1;
#[allow(dead_code)]
pub(crate) const DISPLAY_ERROR_NO_MEMORY: u32 = 2;

pub(crate) struct DisplayInner {
    sources_poll: SourcesPoll,
    clients_mgr: Rc<RefCell<ClientManager>>,
    global_mgr: Rc<RefCell<GlobalManager>>,
    listeners: Vec<SourceInner<FdEvent>>,
}

impl DisplayInner {
    pub(crate) fn new() -> (Rc<RefCell<DisplayInner>>, EventLoopInner) {
        let mut evl = EventLoopInner::new();

        let global_mgr = Rc::new(RefCell::new(GlobalManager::new()));

        let display = Rc::new(RefCell::new(DisplayInner {
            sources_poll: evl.get_poll(),
            clients_mgr: Rc::new(RefCell::new(ClientManager::new(
                evl.get_poll(),
                global_mgr.clone(),
            ))),
            global_mgr,
            listeners: Vec::new(),
        }));

        evl.display = Some(display.clone());

        (display, evl)
    }

    pub(crate) fn create_global<I: Interface, F1, F2>(
        &mut self,
        evl: &EventLoopInner,
        version: u32,
        implementation: F1,
        filter: Option<F2>,
    ) -> GlobalInner<I>
    where
        F1: FnMut(NewResource<I>, u32) + 'static,
        F2: FnMut(ClientInner) -> bool + 'static,
    {
        self.global_mgr
            .borrow_mut()
            .add_global(evl, version, implementation, filter)
    }

    pub(crate) fn flush_clients(&mut self) {
        self.clients_mgr.borrow_mut().flush_all()
    }

    fn add_unix_listener(&mut self, listener: UnixListener) -> io::Result<()> {
        let fd = listener.as_raw_fd();

        listener.set_nonblocking(true)?;

        let mut implem = ListenerImplementation {
            listener,
            client_mgr: self.clients_mgr.clone(),
        };

        let source = self.sources_poll.insert_source(
            fd,
            Ready::readable(),
            move |evt| implem.receive(evt),
            FdEvent::Ready {
                fd,
                mask: FdInterest::READ,
            },
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

struct ListenerImplementation {
    listener: UnixListener,
    client_mgr: Rc<RefCell<ClientManager>>,
}

impl ListenerImplementation {
    fn eprint_error(&self, verb: &str, error: io::Error) {
        if let Ok(addr) = self.listener.local_addr() {
            if let Some(path) = addr.as_pathname() {
                eprintln!(
                    "[wayland-server] Error {} listening socket {} : {}",
                    verb,
                    path.display(),
                    error
                );
                return;
            }
        }
        eprintln!(
            "[wayland-server] Error {} listening socket <unnamed> : {}",
            verb, error
        );
    }

    fn receive(&mut self, event: FdEvent) {
        match event {
            FdEvent::Ready { .. } => {
                // one (or more) clients connected to the socket
                loop {
                    match self.listener.accept() {
                        Ok((stream, _)) => unsafe {
                            self.client_mgr.borrow_mut().init_client(stream.into_raw_fd());
                        },
                        Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                            // we have exhausted all the pending connections
                            break;
                        }
                        Err(e) => {
                            // this is a legitimate error
                            self.eprint_error("accepting connection on", e);
                        }
                    }
                }
            }
            FdEvent::Error { error, .. } => self.eprint_error("polling", error),
        }
    }
}

impl Drop for DisplayInner {
    fn drop(&mut self) {
        for l in self.listeners.drain(..) {
            l.remove();
        }
    }
}

impl Drop for ListenerImplementation {
    fn drop(&mut self) {
        if let Ok(socketaddr) = self.listener.local_addr() {
            if let Some(path) = socketaddr.as_pathname() {
                let _ = ::std::fs::remove_file(path);
            }
        }
    }
}
