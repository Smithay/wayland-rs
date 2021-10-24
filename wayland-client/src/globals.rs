use std::ops::Range;

use crate::{protocol::wl_registry, ConnectionHandle, Dispatch, Proxy, QueueHandle};

pub struct GlobalDescription {
    pub name: u32,
    pub interface: String,
    pub version: u32,
}

pub struct GlobalList {
    globals: Vec<GlobalDescription>,
}

impl Dispatch<wl_registry::WlRegistry> for GlobalList {
    type UserData = ();
    fn event(
        &mut self,
        _: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &mut crate::ConnectionHandle,
        _: &crate::QueueHandle<Self>,
    ) {
        match event {
            wl_registry::Event::Global { name, interface, version } => {
                self.globals.push(GlobalDescription { name, interface, version });
            }
            wl_registry::Event::GlobalRemove { name } => {
                self.globals.retain(|desc| desc.name != name);
            }
        }
    }
}

impl GlobalList {
    pub fn new() -> GlobalList {
        GlobalList { globals: Vec::new() }
    }

    pub fn list(&self) -> &[GlobalDescription] {
        &self.globals
    }

    pub fn bind<I: Proxy + 'static, D: Dispatch<I> + 'static>(
        &self,
        cx: &mut ConnectionHandle<'_>,
        qh: &QueueHandle<D>,
        registry: &wl_registry::WlRegistry,
        version: Range<u32>,
    ) -> Result<I, BindError> {
        for desc in &self.globals {
            if desc.interface != I::interface().name {
                continue;
            }

            if version.contains(&desc.version) {
                return Ok(registry
                    .bind::<I, D>(cx, desc.name, desc.version, qh)
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

#[derive(Debug, thiserror::Error)]
pub enum BindError {
    #[error("Requested global was not advertized by the server: {interface}")]
    MissingGlobal { interface: &'static str },
    #[error("Global {interface} has version {got}, which is outside of the requested range ({requested:?})")]
    WrongVersion { interface: &'static str, requested: Range<u32>, got: u32 },
}
