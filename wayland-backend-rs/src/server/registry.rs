use std::{
    ffi::{CStr, CString},
    sync::Arc,
};

use wayland_commons::{
    server::{GlobalHandler, GlobalInfo, InvalidId, ServerBackend},
    Argument, Interface,
};

use super::{
    client::{Client, ClientStore},
    ClientId, GlobalId, ObjectId,
};

/*
    GlobalId.id is the global protocol name (starting at 1), hence
    we must subtract 1 to it before indexing the vec
*/

struct Global<B> {
    id: GlobalId,
    interface: &'static Interface,
    version: u32,
    handler: Arc<dyn GlobalHandler<B>>,
    disabled: bool,
}

pub struct Registry<B> {
    globals: Vec<Option<Global<B>>>,
    known_registries: Vec<ObjectId>,
    last_serial: u32,
}

impl<B: ServerBackend<ClientId = ClientId, GlobalId = GlobalId, ObjectId = ObjectId>> Registry<B> {
    pub(crate) fn new() -> Self {
        Registry { globals: Vec::new(), known_registries: Vec::new(), last_serial: 0 }
    }

    fn next_serial(&mut self) -> u32 {
        self.last_serial = self.last_serial.wrapping_add(1);
        self.last_serial
    }

    pub(crate) fn create_global(
        &mut self,
        interface: &'static Interface,
        version: u32,
        handler: Arc<dyn GlobalHandler<B>>,
        clients: &mut ClientStore<B>,
    ) -> GlobalId {
        let serial = self.next_serial();
        let (id, place) = match self.globals.iter_mut().enumerate().find(|(_, g)| g.is_none()) {
            Some((id, place)) => (id, place),
            None => {
                self.globals.push(None);
                (self.globals.len() - 1, self.globals.last_mut().unwrap())
            }
        };

        let id = GlobalId { id: id as u32 + 1, serial };

        *place = Some(Global { id, interface, version, handler, disabled: false });

        self.send_global_to_all(id, clients).unwrap();

        id
    }

    fn get_global(&self, id: GlobalId) -> Result<&Global<B>, InvalidId> {
        self.globals
            .get(id.id as usize - 1)
            .and_then(|o| o.as_ref())
            .filter(|o| o.id == id)
            .ok_or(InvalidId)
    }

    pub(crate) fn get_info(&self, id: GlobalId) -> Result<GlobalInfo, InvalidId> {
        let global = self.get_global(id)?;
        Ok(GlobalInfo {
            interface: global.interface,
            version: global.version,
            disabled: global.disabled,
        })
    }

    pub(crate) fn get_handler(&self, id: GlobalId) -> Result<Arc<dyn GlobalHandler<B>>, InvalidId> {
        let global = self.get_global(id)?;
        Ok(global.handler.clone())
    }

    pub(crate) fn check_bind(
        &self,
        client: ClientId,
        name: u32,
        interface_name: &CStr,
        version: u32,
    ) -> Option<(&'static Interface, GlobalId, Arc<dyn GlobalHandler<B>>)> {
        if name == 0 {
            return None;
        }
        let target_global = self.globals.get((name - 1) as usize).and_then(|o| o.as_ref())?;
        if target_global.interface.name.as_bytes() != interface_name.to_bytes() {
            return None;
        }
        if target_global.version < version {
            return None;
        }
        if !target_global.handler.can_view(client, target_global.id) {
            return None;
        }

        Some((target_global.interface, target_global.id, target_global.handler.clone()))
    }

    pub(crate) fn cleanup(&mut self, dead_clients: &[ClientId]) {
        self.known_registries.retain(|obj_id| !dead_clients.contains(&obj_id.client_id))
    }

    pub(crate) fn disable_global(&mut self, id: GlobalId, clients: &mut ClientStore<B>) {
        let global = match self.globals.get_mut(id.id as usize - 1) {
            Some(&mut Some(ref mut g)) if g.id == id => g,
            _ => return,
        };

        // Do nothing if the global is already disabled
        if !global.disabled {
            global.disabled = true;
            // send the global_remove
            for registry in self.known_registries.iter().copied() {
                if let Ok(client) = clients.get_client_mut(registry.client_id) {
                    let _ = send_global_remove_to(client, global, registry);
                }
            }
        }
    }

    pub(crate) fn remove_global(&mut self, id: GlobalId, clients: &mut ClientStore<B>) {
        // disable the global if not already disabled
        self.disable_global(id, clients);
        // now remove it if the id is still valid
        if let Some(place) = self.globals.get_mut(id.id as usize - 1) {
            if place.as_ref().map(|g| g.id == id).unwrap_or(false) {
                *place = None;
            }
        }
    }

    pub(crate) fn send_all_globals_to(
        &self,
        registry: ObjectId,
        client: &mut Client<B>,
    ) -> Result<(), InvalidId> {
        for global in self.globals.iter().flat_map(|opt| opt.as_ref()) {
            if !global.disabled && global.handler.can_view(client.id, global.id) {
                // fail the whole send on error, there is no point in trying further on a failing client
                send_global_to(client, global, registry)?;
            }
        }
        Ok(())
    }

    pub(crate) fn send_global_to_all(
        &self,
        global_id: GlobalId,
        clients: &mut ClientStore<B>,
    ) -> Result<(), InvalidId> {
        let global = self.get_global(global_id)?;
        if global.disabled {
            return Err(InvalidId);
        }
        for registry in self.known_registries.iter().copied() {
            if let Ok(client) = clients.get_client_mut(registry.client_id) {
                // don't fail the whole send for a single erroring client
                let _ = send_global_to(client, global, registry);
            }
        }
        Ok(())
    }
}

#[inline]
fn send_global_to<B>(
    client: &mut Client<B>,
    global: &Global<B>,
    registry: ObjectId,
) -> Result<(), InvalidId>
where
    B: ServerBackend<ObjectId = ObjectId, ClientId = ClientId, GlobalId = GlobalId>,
{
    client.send_event(
        registry,
        0, // wl_registry.global
        &[
            Argument::Uint(global.id.id),
            Argument::Str(Box::new(CString::new(global.interface.name).unwrap())),
            Argument::Uint(global.version),
        ],
    )
}

#[inline]
fn send_global_remove_to<B>(
    client: &mut Client<B>,
    global: &Global<B>,
    registry: ObjectId,
) -> Result<(), InvalidId>
where
    B: ServerBackend<ObjectId = ObjectId, ClientId = ClientId, GlobalId = GlobalId>,
{
    client.send_event(
        registry,
        1, // wl_registry.global_remove
        &[Argument::Uint(global.id.id)],
    )
}
