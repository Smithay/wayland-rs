use std::os::unix::io::RawFd;

use super::event_loop::SourcesPoll;

#[derive(Clone)]
pub(crate) struct ClientInner {}

impl ClientInner {
    pub(crate) fn alive(&self) -> bool {
        unimplemented!()
    }

    pub(crate) fn equals(&self, other: &ClientInner) -> bool {
        unimplemented!()
    }

    pub(crate) fn flush(&self) {
        unimplemented!()
    }

    pub(crate) fn kill(&self) {
        unimplemented!()
    }

    pub(crate) fn set_user_data(&self, data: *mut ()) {
        unimplemented!()
    }

    pub(crate) fn get_user_data(&self) -> *mut () {
        unimplemented!()
    }

    pub(crate) fn set_destructor(&self, destructor: fn(*mut ())) {
        unimplemented!()
    }
}

pub(crate) struct ClientManager {
    sources_poll: SourcesPoll,
}

impl ClientManager {
    pub(crate) fn new(sources_poll: SourcesPoll) -> ClientManager {
        ClientManager { sources_poll }
    }

    pub(crate) unsafe fn init_client(&mut self, fd: RawFd) -> ClientInner {
        unimplemented!()
    }

    pub(crate) fn flush_all(&mut self) {
        unimplemented!()
    }
}
