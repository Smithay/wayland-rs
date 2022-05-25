//! Helpers for listing and bindings globals

use std::ops::Range;

use crate::{protocol::wl_registry, Connection, DelegateDispatch, Dispatch, Proxy, QueueHandle};

/// Description of an advertized global
#[derive(Debug)]
pub struct GlobalDescription {
    /// identifier of this global
    pub name: u32,
    /// interface name
    pub interface: String,
    /// advertized version
    pub version: u32,
}

/// A helper to retrieve a list of globals and bind them
///
/// The `GlobalList` can be used as a [`Dispatch`](crate::Dispatch) target for the `wl_registry`. It
/// maintains a list of globals advertized by the compositor, and provides a way to bind according to
/// specified version requirements. It is an easy way to ensure at startup that the server advertized
/// all the globals your app needs, and bind them all at once.
#[derive(Debug)]
pub struct GlobalList {
    globals: Vec<GlobalDescription>,
}

impl<D> DelegateDispatch<wl_registry::WlRegistry, (), D> for GlobalList
where
    D: Dispatch<wl_registry::WlRegistry, ()> + AsMut<GlobalList>,
{
    fn event(
        handle: &mut D,
        _: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        _: &crate::QueueHandle<D>,
    ) {
        let me = handle.as_mut();
        match event {
            wl_registry::Event::Global { name, interface, version } => {
                me.globals.push(GlobalDescription { name, interface, version });
            }
            wl_registry::Event::GlobalRemove { name } => {
                me.globals.retain(|desc| desc.name != name);
            }
        }
    }
}

impl AsMut<GlobalList> for GlobalList {
    fn as_mut(&mut self) -> &mut GlobalList {
        self
    }
}

impl Dispatch<wl_registry::WlRegistry, ()> for GlobalList {
    #[inline]
    fn event(
        &mut self,
        proxy: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        data: &(),
        conn: &Connection,
        qhandle: &QueueHandle<Self>,
    ) {
        <Self as DelegateDispatch<wl_registry::WlRegistry, (), Self>>::event(
            self, proxy, event, data, conn, qhandle,
        )
    }
}

impl Default for GlobalList {
    fn default() -> Self {
        Self::new()
    }
}

impl GlobalList {
    /// Create a new `GLobalList`
    pub fn new() -> Self {
        Self { globals: Vec::new() }
    }

    /// Access the list of currently advertized globals
    pub fn list(&self) -> &[GlobalDescription] {
        &self.globals
    }

    /// Bind a global
    ///
    /// You can specify the requested interface as type parameter, and the version range. You
    /// also need to provide the user data value that will be set for the newly created object.
    pub fn bind<I: Proxy + 'static, U: Send + Sync + 'static, D: Dispatch<I, U> + 'static>(
        &self,
        qh: &QueueHandle<D>,
        registry: &wl_registry::WlRegistry,
        version: Range<u32>,
        user_data: U,
    ) -> Result<I, BindError> {
        for desc in &self.globals {
            if desc.interface != I::interface().name {
                continue;
            }

            if version.contains(&desc.version) {
                return Ok(registry
                    .bind::<I, U, D>(desc.name, desc.version, qh, user_data)
                    .expect("invalid wl_registry"));
            } else {
                return Err(BindError::WrongVersion {
                    interface: I::interface().name,
                    requested: version,
                    got: desc.version,
                });
            }
        }

        Err(BindError::MissingGlobal { interface: I::interface().name })
    }
}

/// Error when trying to bind a global
#[derive(Debug, thiserror::Error)]
pub enum BindError {
    /// The requested global was not advertized by the server
    #[error("Requested global was not advertized by the server: {interface}")]
    MissingGlobal {
        /// The requested interface
        interface: &'static str,
    },
    /// The version advertized by the server did not fit in the requested range
    #[error("Global {interface} has version {got}, which is outside of the requested range ({requested:?})")]
    WrongVersion {
        /// The requested interface
        interface: &'static str,
        /// The requested version range
        requested: Range<u32>,
        /// The advertized version
        got: u32,
    },
}
