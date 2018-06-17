use std::collections::VecDeque;
use std::io;
use std::sync::{Arc, Mutex};

use wayland_commons::wire::Message;

use super::connection::{Connection, Error as CError};

pub(crate) type QueueBuffer = Arc<Mutex<VecDeque<Message>>>;

pub(crate) fn create_queue_buffer() -> QueueBuffer {
    Arc::new(Mutex::new(VecDeque::new()))
}

pub(crate) struct EventQueueInner {
    connection: Arc<Mutex<Connection>>,
    buffer: QueueBuffer,
}

impl EventQueueInner {
    pub(crate) fn new(connection: Arc<Mutex<Connection>>, buffer: Option<QueueBuffer>) -> EventQueueInner {
        EventQueueInner {
            connection,
            buffer: buffer.unwrap_or_else(create_queue_buffer),
        }
    }

    pub(crate) fn dispatch(&mut self) -> io::Result<u32> {
        // don't read events if there are some pending
        if let Err(()) = self.prepare_read() {
            return self.dispatch_pending();
        }

        // TODO: flush the display

        // TODO: block on wait for read readiness before reading
        self.read_events()?;

        self.dispatch_pending()
    }

    pub(crate) fn dispatch_pending(&mut self) -> io::Result<u32> {
        unimplemented!()
    }

    pub(crate) fn sync_roundtrip(&mut self) -> io::Result<i32> {
        unimplemented!()
    }

    pub(crate) fn prepare_read(&self) -> Result<(), ()> {
        // TODO: un-mock
        Ok(())
    }

    pub(crate) fn read_events(&self) -> io::Result<i32> {
        // TODO: integrate more properly with prepare read with a fence
        match self.connection.lock().unwrap().read_events() {
            Ok(n) => Ok(n as i32),
            Err(CError::Protocol) => Err(::nix::errno::Errno::EPROTO.into()),
            Err(CError::Parse(_)) => Err(::nix::errno::Errno::EPROTO.into()),
            Err(CError::Nix(::nix::Error::Sys(errno))) => Err(errno.into()),
            Err(CError::Nix(_)) => unreachable!(),
        }
    }

    pub(crate) fn cancel_read(&self) {
        // TODO: un-mock
    }
}
