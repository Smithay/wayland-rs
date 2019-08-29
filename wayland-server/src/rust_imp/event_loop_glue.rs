use std::cell::RefCell;
use std::io;
use std::os::unix::io::AsRawFd;
use std::os::unix::net::{UnixListener, UnixStream};
use std::rc::Rc;

use calloop::generic::{Event, Generic};
use calloop::{EventDispatcher, EventSource, LoopHandle, Source};

use mio::{Evented, Poll, PollOpt, Ready, Token};

use crate::Fd;

pub(crate) trait WSLoopHandle {
    fn add_listener(
        &self,
        source: WaylandListener,
        cb: Box<dyn FnMut(UnixStream)>,
    ) -> io::Result<Source<WaylandListener>>;

    fn add_socket(
        &self,
        source: Generic<Fd>,
        cb: Box<dyn FnMut(Event<Fd>)>,
    ) -> io::Result<Source<Generic<Fd>>>;
}

impl<Data: 'static> WSLoopHandle for LoopHandle<Data> {
    fn add_listener(
        &self,
        source: WaylandListener,
        mut cb: Box<dyn FnMut(UnixStream)>,
    ) -> io::Result<Source<WaylandListener>> {
        self.insert_source(source, move |evt, _| cb(evt))
            .map_err(Into::<io::Error>::into)
    }

    fn add_socket(
        &self,
        source: Generic<Fd>,
        mut cb: Box<dyn FnMut(Event<Fd>)>,
    ) -> io::Result<Source<Generic<Fd>>> {
        self.insert_source(source, move |evt, _| cb(evt))
            .map_err(Into::<io::Error>::into)
    }
}

/*
 * Event source for listener socket
 */

pub(crate) struct WaylandListener {
    listener: Rc<RefCell<UnixListener>>,
}

impl WaylandListener {
    pub(crate) fn new(listener: UnixListener) -> WaylandListener {
        WaylandListener {
            listener: Rc::new(RefCell::new(listener)),
        }
    }
}

impl Evented for WaylandListener {
    fn register(&self, poll: &Poll, token: Token, interest: Ready, opts: PollOpt) -> io::Result<()> {
        ::mio::unix::EventedFd(&self.listener.borrow().as_raw_fd()).register(poll, token, interest, opts)
    }

    fn reregister(&self, poll: &Poll, token: Token, interest: Ready, opts: PollOpt) -> io::Result<()> {
        ::mio::unix::EventedFd(&self.listener.borrow().as_raw_fd()).reregister(poll, token, interest, opts)
    }

    fn deregister(&self, poll: &Poll) -> io::Result<()> {
        ::mio::unix::EventedFd(&self.listener.borrow().as_raw_fd()).deregister(poll)
    }
}

impl EventSource for WaylandListener {
    type Event = UnixStream;

    fn interest(&self) -> Ready {
        Ready::readable()
    }
    fn pollopts(&self) -> PollOpt {
        PollOpt::edge()
    }
    fn make_dispatcher<Data: 'static, F: FnMut(Self::Event, &mut Data) + 'static>(
        &self,
        callback: F,
    ) -> Rc<RefCell<dyn EventDispatcher<Data>>> {
        struct Dispatcher<F> {
            listener: Rc<RefCell<UnixListener>>,
            callback: F,
        }

        impl<Data, F: FnMut(UnixStream, &mut Data)> EventDispatcher<Data> for Dispatcher<F> {
            fn ready(&mut self, _: Ready, data: &mut Data) {
                let listener = self.listener.borrow_mut();
                loop {
                    match listener.accept() {
                        Ok((stream, _)) => (self.callback)(stream, data),
                        Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                            // we have exhausted all the pending connections
                            break;
                        }
                        Err(e) => {
                            // this is a legitimate error
                            eprint_error(&listener, e);
                        }
                    }
                }
            }
        }

        Rc::new(RefCell::new(Dispatcher {
            callback,
            listener: self.listener.clone(),
        }))
    }
}

fn eprint_error(listener: &UnixListener, error: io::Error) {
    if let Ok(addr) = listener.local_addr() {
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

impl Drop for WaylandListener {
    fn drop(&mut self) {
        if let Ok(socketaddr) = self.listener.borrow().local_addr() {
            if let Some(path) = socketaddr.as_pathname() {
                let _ = ::std::fs::remove_file(path);
            }
        }
    }
}
