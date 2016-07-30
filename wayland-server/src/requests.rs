use std::iter::Iterator;
use std::ops::Deref;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use {ResourceId, ClientId};
use globals::GlobalId;

/// All possible wayland requests.
///
/// This enum does a first sorting of request, with a variant for each
/// protocol extension activated in cargo features.
///
/// As the number of variant of this enum can change depending on which cargo features are
/// activated, it should *never* be matched exhaustively. It contains an hidden, never-used
/// variant to ensure it.
///
/// However, you should always match it for *all* the request associated with resources
/// derived from globals that have previously been advertized, in order to respect the
/// wayland protocol.
#[derive(Debug)]
pub enum Request {
    Wayland(::wayland::WaylandProtocolRequest),
    #[cfg(all(feature = "unstable-protocols", feature="wpu-xdg_shell"))]
    XdgShellUnstableV5(::xdg_shell::XdgShellUnstableV5ProtocolRequest),
    #[cfg(feature="wl-desktop_shell")]
    Desktop(::desktop_shell::DesktopProtocolRequest),
    #[doc(hidden)]
    __DoNotMatchThis,
}

pub type RequestFifo = ::crossbeam::sync::MsQueue<Request>;

pub struct RequestIterator {
    fifo: Arc<(RequestFifo, AtomicBool)>
}

impl RequestIterator {
    pub fn new() -> RequestIterator {
        RequestIterator {
            fifo: Arc::new((::crossbeam::sync::MsQueue::new(),AtomicBool::new(true)))
        }
    }
}

impl Drop for RequestIterator {
    fn drop(&mut self) {
        self.fifo.1.store(false, Ordering::SeqCst);
    }
}

impl Iterator for RequestIterator {
    type Item = Request;
    fn next(&mut self) -> Option<Request> {
        self.fifo.0.try_pop()
    }
}

pub fn get_requestiter_internals(evt: &RequestIterator) -> Arc<(RequestFifo, AtomicBool)> {
    evt.fifo.clone()
}

pub enum ResourceParent {
    Global(GlobalId),
    Resource(ResourceId)
}

pub trait IteratorDispatch {
    fn get_iterator(&self, client: ClientId, parent: ResourceParent) -> &RequestIterator;
}

impl<T> IteratorDispatch for T where T: Deref<Target=RequestIterator> {
    fn get_iterator(&self, _: ClientId, _: ResourceParent) -> &RequestIterator {
        self.deref()
    }
}
