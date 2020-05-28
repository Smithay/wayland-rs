use std::cell::RefCell;
use std::os::unix::io::RawFd;

use nix::sys::epoll::*;

use crate::DispatchData;

type FdData = (RawFd, Option<Box<dyn FnMut(crate::DispatchData<'_>)>>);

#[derive(Copy, Clone)]
pub(crate) struct Token(usize);

pub(crate) struct FdManager {
    epoll_fd: RawFd,
    callbacks: RefCell<Vec<Option<FdData>>>,
}

impl FdManager {
    pub(crate) fn new() -> nix::Result<FdManager> {
        let fd = epoll_create1(EpollCreateFlags::EPOLL_CLOEXEC)?;

        Ok(FdManager { epoll_fd: fd, callbacks: RefCell::new(Vec::new()) })
    }

    pub(crate) fn register<F: FnMut(DispatchData<'_>) + 'static>(
        &self,
        fd: RawFd,
        cb: F,
    ) -> nix::Result<Token> {
        let mut callbacks = self.callbacks.borrow_mut();
        // find the first free id
        let free_id = callbacks.iter().position(|c| c.is_none());
        let cb = Some(Box::new(cb) as Box<dyn FnMut(DispatchData<'_>)>);
        let id = match free_id {
            Some(i) => {
                callbacks[i] = Some((fd, cb));
                i
            }
            None => {
                callbacks.push(Some((fd, cb)));
                callbacks.len() - 1
            }
        };
        let mut evt = EpollEvent::new(EpollFlags::EPOLLIN, id as u64);
        let ret = epoll_ctl(self.epoll_fd, EpollOp::EpollCtlAdd, fd, &mut evt);
        match ret {
            Ok(()) => Ok(Token(id)),
            Err(e) => {
                callbacks[id] = None;
                Err(e)
            }
        }
    }

    pub(crate) fn deregister(&self, token: Token) {
        if let Some((fd, _)) = self.callbacks.borrow_mut()[token.0].take() {
            let _ = epoll_ctl(self.epoll_fd, EpollOp::EpollCtlDel, fd, None);
        }
    }

    pub(crate) fn poll(&self, timeout: i32, mut data: crate::DispatchData) -> nix::Result<()> {
        let mut events = [EpollEvent::empty(); 32];
        let n = epoll_wait(self.epoll_fd, &mut events, timeout as isize)?;

        for event in events.iter().take(n) {
            let id = event.data() as usize;
            // remove the cb while we call it, to gracefully handle reentrancy
            let cb = self.callbacks.borrow_mut()[id].as_mut().and_then(|(_, ref mut cb)| cb.take());
            if let Some(mut cb) = cb {
                cb(data.reborrow());
                // now, put it back in place
                if let Some(ref mut place) = self.callbacks.borrow_mut()[id] {
                    if place.1.is_none() {
                        place.1 = Some(cb)
                    }
                    // if there is already something here, this means that our callback has been
                    // deleted and replaced by a new one while `cb` was running, in which case we should
                    // not put it back in place
                }
            // If self.callbacks[id] is None, this means that our callback has been deleted while running,
            // in which case we should not put it back in place
            } else {
                panic!("Received a readiness for a deregistered fd?!");
            }
        }

        Ok(())
    }

    pub(crate) fn get_poll_fd(&self) -> RawFd {
        self.epoll_fd
    }
}
