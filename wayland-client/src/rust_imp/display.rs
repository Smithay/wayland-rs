use std::io;
use std::os::unix::io::RawFd;
use std::sync::{Arc, Mutex};

use wayland_commons::map::Object;

use protocol::wl_display::WlDisplay;

use {ConnectError, Proxy};

use super::connection::Connection;
use super::proxy::{ObjectMeta, ProxyInner};
use super::EventQueueInner;

pub(crate) struct DisplayInner {
    connection: Arc<Mutex<Connection>>,
    proxy: Proxy<WlDisplay>,
}

impl DisplayInner {
    pub unsafe fn from_fd(fd: RawFd) -> Result<(Arc<DisplayInner>, EventQueueInner), ConnectError> {
        let buffer = super::queues::create_queue_buffer();
        let display_object = Object::from_interface::<WlDisplay>(1, ObjectMeta::new(buffer.clone()));
        let (connection, map) = {
            let c = Connection::new(fd, display_object);
            let m = c.map.clone();
            (Arc::new(Mutex::new(c)), m)
        };
        let event_queue = EventQueueInner::new(connection.clone(), Some(buffer));
        let display = DisplayInner {
            proxy: Proxy::wrap(ProxyInner::from_id(1, map, connection.clone()).unwrap()),
            connection,
        };
        Ok((Arc::new(display), event_queue))
    }

    pub(crate) fn flush(&self) -> io::Result<()> {
        match self.connection.lock().unwrap().flush() {
            Ok(()) => Ok(()),
            Err(::nix::Error::Sys(errno)) => Err(errno.into()),
            Err(_) => unreachable!(),
        }
    }

    pub(crate) fn create_event_queue(me: &Arc<DisplayInner>) -> EventQueueInner {
        EventQueueInner::new(me.connection.clone(), None)
    }

    pub(crate) fn get_proxy(&self) -> &Proxy<WlDisplay> {
        &self.proxy
    }
}
