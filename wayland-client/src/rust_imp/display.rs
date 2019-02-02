use std::io;
use std::os::unix::io::RawFd;
use std::sync::{Arc, Mutex};

use wayland_commons::map::Object;
use wayland_commons::utils::UserData;

use protocol::wl_display::{self, WlDisplay};

use {ConnectError, ProtocolError, Proxy};

use super::connection::{Connection, Error as CxError};
use super::proxy::{NewProxyInner, ObjectMeta};
use super::EventQueueInner;

pub(crate) struct DisplayInner {
    connection: Arc<Mutex<Connection>>,
    proxy: WlDisplay,
}

impl DisplayInner {
    pub unsafe fn from_fd(fd: RawFd) -> Result<(Arc<DisplayInner>, EventQueueInner), ConnectError> {
        // The special buffer for display events
        let buffer = super::queues::create_queue_buffer();
        let display_object = Object::from_interface::<WlDisplay>(1, ObjectMeta::new(buffer.clone()));
        let (connection, map) = {
            let c = Connection::new(fd, display_object);
            let m = c.map.clone();
            (Arc::new(Mutex::new(c)), m)
        };

        let display_newproxy = NewProxyInner::from_id(1, map.clone(), connection.clone()).unwrap();

        // give access to the map to the display impl
        let impl_map = map;
        let impl_last_error = connection.lock().unwrap().last_error.clone();
        // our implementation is Send, we are safe
        let display_proxy = display_newproxy.implement::<WlDisplay, _>(
            move |event, _| match event {
                wl_display::Event::Error {
                    object_id,
                    code,
                    message,
                } => {
                    eprintln!(
                        "[wayland-client] Protocol error {} on object {}@{}: {}",
                        code,
                        object_id.as_ref().inner.object.interface,
                        object_id.as_ref().id(),
                        message
                    );
                    *impl_last_error.lock().unwrap() = Some(CxError::Protocol(ProtocolError {
                        code,
                        object_id: object_id.as_ref().id(),
                        object_interface: object_id.as_ref().inner.object.interface,
                        message,
                    }));
                }
                wl_display::Event::DeleteId { id } => {
                    // cleanup the map as appropriate
                    let mut map = impl_map.lock().unwrap();
                    let client_destroyed = map
                        .with(id, |obj| {
                            obj.meta.server_destroyed = true;
                            obj.meta.client_destroyed
                        })
                        .unwrap_or(false);
                    if client_destroyed {
                        map.remove(id);
                    }
                }
                _ => {}
            },
            UserData::empty(),
        );

        let default_event_queue = EventQueueInner::new(connection.clone(), None);

        let display = DisplayInner {
            proxy: Proxy::wrap(display_proxy.make_wrapper(&default_event_queue).unwrap()).into(),
            connection,
        };

        Ok((Arc::new(display), default_event_queue))
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

    pub(crate) fn get_proxy(&self) -> &WlDisplay {
        &self.proxy
    }

    pub(crate) fn protocol_error(&self) -> Option<ProtocolError> {
        let cx = self.connection.lock().unwrap();
        let last_error = cx.last_error.lock().unwrap();
        if let Some(CxError::Protocol(ref e)) = *last_error {
            Some(e.clone())
        } else {
            None
        }
    }
}
