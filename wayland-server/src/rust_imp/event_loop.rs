use std::any::Any;
use std::cell::RefCell;
use std::io;
use std::os::unix::io::RawFd;
use std::rc::Rc;
use std::time::Duration;

use mio::unix::EventedFd;
use mio::{Events, Poll, PollOpt, Ready, Token};

use sources::*;

use super::DisplayInner;

pub(crate) struct EventLoopInner {
    pub(crate) display: Option<Rc<RefCell<DisplayInner>>>,
    sources_poll: SourcesPoll,
    idles: RefCell<Vec<Rc<RefCell<Option<Box<FnMut()>>>>>>,
}

#[derive(Clone)]
pub(crate) struct SourcesPoll {
    poll: Rc<Poll>,
    sources: Rc<RefCell<SourceList>>,
}

impl SourcesPoll {
    fn dispatch(&self, timeout: Option<u32>) -> io::Result<()> {
        let mut evts = Events::with_capacity(32);

        self.poll
            .poll(&mut evts, timeout.map(|s| Duration::from_millis(s as u64)))?;

        for evt in evts {
            let Token(idx) = evt.token();
            let dispatcher = self.sources.borrow().get_dispatcher(idx);
            if let Some(dispatcher) = dispatcher {
                dispatcher.borrow_mut().ready(evt.readiness());
            }
        }

        Ok(())
    }

    pub(crate) fn insert_source<F, E>(
        &self,
        fd: RawFd,
        interest: Ready,
        implementation: F,
        evt: E,
    ) -> Result<SourceInner<E>, io::Error>
    where
        F: FnMut(E) + 'static,
        SourceDispatcher<E>: EventDispatcher,
    {
        let implem = Rc::new(RefCell::new(SourceDispatcher {
            fd,
            implem: Box::new(implementation),
            evt,
        }));
        let token = self.sources.borrow_mut().add_source(fd, implem.clone());
        let source = SourceInner {
            dispatcher: implem,
            list: self.sources.clone(),
            poll: self.poll.clone(),
            token,
            fd,
        };
        self.poll
            .register(&EventedFd(&fd), token, interest, PollOpt::empty())?;
        Ok(source)
    }
}

impl EventLoopInner {
    pub(crate) fn new() -> EventLoopInner {
        EventLoopInner {
            display: None,
            sources_poll: SourcesPoll {
                poll: Rc::new(Poll::new().unwrap()),
                sources: Rc::new(RefCell::new(SourceList::new())),
            },
            idles: RefCell::new(Vec::new()),
        }
    }

    pub(crate) fn get_poll(&self) -> SourcesPoll {
        self.sources_poll.clone()
    }

    pub(crate) fn dispatch(&self, timeout: Option<u32>) -> io::Result<u32> {
        self.dispatch_idles();

        self.sources_poll.dispatch(timeout)?;

        self.dispatch_idles();

        Ok(0)
    }

    pub(crate) fn flush_clients_if_display(&self) {
        if let Some(ref display) = self.display {
            display.borrow_mut().flush_clients();
        }
    }

    pub(crate) fn add_fd_event_source<F>(
        &self,
        fd: RawFd,
        interest: FdInterest,
        implementation: F,
    ) -> Result<SourceInner<FdEvent>, io::Error>
    where
        F: FnMut(FdEvent) + 'static,
    {
        self.sources_poll.insert_source(
            fd,
            interest.into(),
            implementation,
            FdEvent::Ready { fd, mask: interest },
        )
    }

    pub(crate) fn add_timer_event_source<F>(
        &self,
        implementation: F,
    ) -> Result<SourceInner<TimerEvent>, io::Error>
    where
        F: FnMut(TimerEvent) + 'static,
    {
        use libc::{timerfd_create, CLOCK_MONOTONIC, TFD_CLOEXEC, TFD_NONBLOCK};
        let fd = unsafe { timerfd_create(CLOCK_MONOTONIC, TFD_CLOEXEC | TFD_NONBLOCK) };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        self.sources_poll
            .insert_source(fd, Ready::readable(), implementation, TimerEvent)
    }

    pub(crate) fn add_signal_event_source<F>(
        &self,
        signal: ::nix::sys::signal::Signal,
        implementation: F,
    ) -> Result<SourceInner<SignalEvent>, io::Error>
    where
        F: FnMut(SignalEvent) + 'static,
    {
        use nix::sys::signal::SigSet;
        use nix::sys::signalfd::{signalfd, SfdFlags};

        let mut set = SigSet::empty();
        set.add(signal);

        let fd = match signalfd(-1, &set, SfdFlags::SFD_NONBLOCK | SfdFlags::SFD_CLOEXEC) {
            Ok(fd) => fd,
            Err(::nix::Error::Sys(e)) => return Err(e.into()),
            Err(_) => unreachable!(),
        };

        self.sources_poll
            .insert_source(fd, Ready::readable(), implementation, SignalEvent(signal))
    }

    pub(crate) fn add_idle_event_source<F>(&self, implementation: F) -> IdleSourceInner
    where
        F: FnMut() + 'static,
    {
        let implem = Rc::new(RefCell::new(Some(Box::new(implementation) as Box<_>)));
        self.idles.borrow_mut().push(implem.clone());
        IdleSourceInner { implem }
    }

    fn dispatch_idles(&self) {
        let idles = ::std::mem::replace(&mut *self.idles.borrow_mut(), Vec::new());
        for idle in idles {
            if let Some(ref mut implem) = *idle.borrow_mut() {
                implem();
            }
        }
    }
}

// SourceList

pub(crate) trait EventDispatcher: Any {
    fn ready(&mut self, ready: Ready);
    fn error(&mut self, error: io::Error);
}

struct SourceList {
    sources: Vec<Option<(RawFd, Rc<RefCell<EventDispatcher>>)>>,
}

impl SourceList {
    fn new() -> SourceList {
        SourceList { sources: Vec::new() }
    }

    fn get_dispatcher(&self, idx: usize) -> Option<Rc<RefCell<EventDispatcher>>> {
        match self.sources.get(idx) {
            Some(&Some((_, ref dispatcher))) => Some(dispatcher.clone()),
            _ => None,
        }
    }

    fn add_source(&mut self, fd: RawFd, source: Rc<RefCell<EventDispatcher>>) -> Token {
        let free_id = self.sources.iter().position(Option::is_none);
        if let Some(id) = free_id {
            self.sources[id] = Some((fd, source));
            Token(id)
        } else {
            self.sources.push(Some((fd, source)));
            Token(self.sources.len() - 1)
        }
    }

    fn del_source(&mut self, source: Rc<RefCell<EventDispatcher>>) {
        for src in &mut self.sources {
            let found = if let &mut Some(ref s) = src {
                Rc::ptr_eq(&source, &s.1)
            } else {
                false
            };
            if found {
                *src = None;
                return;
            }
        }
    }
}

// Sources

pub(crate) struct SourceDispatcher<E> {
    implem: Box<FnMut(E)>,
    fd: RawFd,
    evt: E,
}

pub(crate) struct SourceInner<E> {
    dispatcher: Rc<RefCell<SourceDispatcher<E>>>,
    list: Rc<RefCell<SourceList>>,
    poll: Rc<Poll>,
    token: Token,
    fd: RawFd,
}

impl<E: 'static> SourceInner<E>
where
    SourceDispatcher<E>: EventDispatcher,
{
    pub(crate) fn remove(self) {
        let _ = self.poll.deregister(&EventedFd(&self.fd));
        self.list
            .borrow_mut()
            .del_source(self.dispatcher.clone() as Rc<_>);
    }
}

// FD event source

impl SourceInner<FdEvent> {
    pub(crate) fn update_mask(&mut self, mask: FdInterest) {
        let _ = self
            .poll
            .reregister(&EventedFd(&self.fd), self.token, mask.into(), PollOpt::empty());
    }
}

impl EventDispatcher for SourceDispatcher<FdEvent> {
    fn ready(&mut self, ready: Ready) {
        (self.implem)(FdEvent::Ready {
            fd: self.fd,
            mask: ready.into(),
        });
    }

    fn error(&mut self, error: io::Error) {
        (self.implem)(FdEvent::Error { fd: self.fd, error });
    }
}

// Timer event source

impl SourceInner<TimerEvent> {
    pub(crate) fn set_delay_ms(&mut self, delay: i32) {
        use libc::{c_long, itimerspec, time_t, timerfd_settime, timespec};

        let spec = itimerspec {
            it_interval: timespec {
                tv_sec: 0,
                tv_nsec: 0,
            },
            it_value: timespec {
                tv_sec: delay as time_t / 1000,
                tv_nsec: (delay as c_long % 1000) * 1000 * 1000,
            },
        };

        unsafe {
            timerfd_settime(self.fd, 0, &spec, ::std::ptr::null_mut());
        }
    }
}

impl EventDispatcher for SourceDispatcher<TimerEvent> {
    fn ready(&mut self, _: Ready) {
        (self.implem)(self.evt);
    }

    fn error(&mut self, _: io::Error) {}
}

// Signal event source

impl EventDispatcher for SourceDispatcher<SignalEvent> {
    fn ready(&mut self, _: Ready) {
        (self.implem)(self.evt);
    }

    fn error(&mut self, _: io::Error) {}
}

// Idle event source

pub(crate) struct IdleSourceInner {
    implem: Rc<RefCell<Option<Box<FnMut()>>>>,
}

impl IdleSourceInner {
    pub(crate) fn remove(self) {
        self.implem.borrow_mut().take();
    }
}

impl From<FdInterest> for Ready {
    fn from(mask: FdInterest) -> Ready {
        let mut interest = Ready::empty();
        if mask.contains(FdInterest::READ) {
            interest.insert(Ready::readable())
        }
        if mask.contains(FdInterest::WRITE) {
            interest.insert(Ready::writable())
        }
        interest
    }
}

impl From<Ready> for FdInterest {
    fn from(ready: Ready) -> FdInterest {
        let mut mask = FdInterest::empty();
        if ready.contains(Ready::readable()) {
            mask |= FdInterest::READ
        }
        if ready.contains(Ready::writable()) {
            mask |= FdInterest::WRITE
        }
        mask
    }
}
