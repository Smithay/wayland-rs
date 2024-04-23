//! Helpers for listing and bindings globals

use std::ops::Range;

use wayland_client::{protocol::wl_registry, Connection, Dispatch, Proxy, QueueHandle};

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
/// The `GlobalList` can be used as a [`Dispatch`] target for the `wl_registry`. It
/// maintains a list of globals advertized by the compositor, and provides a way to bind according to
/// specified version requirements. It is an easy way to ensure at startup that the server advertized
/// all the globals your app needs, and bind them all at once.
#[derive(Debug)]
pub struct GlobalList {
    globals: Vec<GlobalDescription>,
}

impl<D> Dispatch<wl_registry::WlRegistry, (), D> for GlobalList
where
    D: Dispatch<wl_registry::WlRegistry, ()> + AsMut<GlobalList>,
{
    fn event(
        handle: &mut D,
        _: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<D>,
    ) {
        let me = handle.as_mut();
        match event {
            wl_registry::Event::Global { name, interface, version } => {
                me.globals.push(GlobalDescription { name, interface, version });
            }
            wl_registry::Event::GlobalRemove { name } => {
                me.globals.retain(|desc| desc.name != name);
            }
            _ => {
                unreachable!()
            }
        }
    }
}

impl AsMut<GlobalList> for GlobalList {
    fn as_mut(&mut self) -> &mut GlobalList {
        self
    }
}

impl Default for GlobalList {
    fn default() -> Self {
        Self::new()
    }
}

impl GlobalList {
    /// Create a new `GlobalList`
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
                return Ok(registry.bind::<I, U, D>(desc.name, desc.version, qh, user_data));
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

#[derive(Debug)]
pub enum BindError {
    MissingGlobal { interface: &'static str },
    WrongVersion { interface: &'static str, requested: Range<u32>, got: u32 },
}

impl std::error::Error for BindError {}

impl std::fmt::Display for BindError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BindError::MissingGlobal { interface } => {
                write!(f, "Requested global was not advertized by the server: {interface}")
            }
            BindError::WrongVersion { interface, requested, got } => {
                write!(f, "Global {interface} has version {got}, which is outside of the requested range ({requested:?})")
            }
        }
    }
}
