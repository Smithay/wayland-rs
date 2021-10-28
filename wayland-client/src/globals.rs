use std::ops::Range;

use crate::{
    event_queue::DataInit, protocol::wl_registry, ConnectionHandle, DelegateDispatch,
    DelegateDispatchBase, Dispatch, Proxy, QueueHandle,
};

pub struct GlobalDescription {
    pub name: u32,
    pub interface: String,
    pub version: u32,
}

pub struct GlobalList {
    globals: Vec<GlobalDescription>,
}

impl DelegateDispatchBase<wl_registry::WlRegistry> for GlobalList {
    type UserData = ();
}

impl<D> DelegateDispatch<wl_registry::WlRegistry, D> for GlobalList
where
    D: Dispatch<wl_registry::WlRegistry, UserData = ()>,
{
    fn event(
        &mut self,
        _: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &mut crate::ConnectionHandle,
        _: &crate::QueueHandle<D>,
        _: &mut crate::DataInit<'_>,
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

impl Dispatch<wl_registry::WlRegistry> for GlobalList {
    type UserData = ();

    #[inline]
    fn event(
        &mut self,
        proxy: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        data: &Self::UserData,
        cxhandle: &mut ConnectionHandle,
        qhandle: &QueueHandle<Self>,
        init: &mut DataInit<'_>,
    ) {
        <Self as DelegateDispatch<wl_registry::WlRegistry, Self>>::event(
            self, proxy, event, data, cxhandle, qhandle, init,
        )
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
        user_data: <D as Dispatch<I>>::UserData,
    ) -> Result<I, BindError> {
        for desc in &self.globals {
            if desc.interface != I::interface().name {
                continue;
            }

            if version.contains(&desc.version) {
                return Ok(registry
                    .bind::<I, D>(cx, desc.name, desc.version, qh, user_data)
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
