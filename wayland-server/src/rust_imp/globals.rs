use std::cell::{Cell, RefCell};
use std::ffi::CString;
use std::rc::Rc;

use wayland_commons::map::Object;
use wayland_commons::smallvec;
use wayland_commons::wire::{Argument, Message};

use crate::{DispatchData, Interface, Main, Resource};

use super::resources::ObjectMeta;
use super::{ClientInner, ResourceInner};

type GlobalFilter = Rc<RefCell<dyn FnMut(ClientInner) -> bool>>;

pub(crate) struct GlobalInner<I: Interface> {
    _i: ::std::marker::PhantomData<*const I>,
    destroyed_marker: Rc<Cell<bool>>,
    id: u32,
    registries: Rc<RefCell<Vec<(u32, ClientInner)>>>,
    filter: Option<GlobalFilter>,
}

impl<I: Interface> GlobalInner<I> {
    pub fn destroy(self) {
        self.destroyed_marker.set(true);
        send_destroyed_global(
            &self.registries.borrow(),
            self.id,
            self.filter.as_ref().map(|f| &**f),
        );
    }
}

type GlobalImplementation = dyn Fn(u32, u32, ClientInner, DispatchData) -> Result<(), ()>;

struct GlobalData {
    version: u32,
    interface: &'static str,
    destroyed: Rc<Cell<bool>>,
    implem: Box<GlobalImplementation>,
    filter: Option<GlobalFilter>,
}

pub(crate) struct GlobalManager {
    registries: Rc<RefCell<Vec<(u32, ClientInner)>>>,
    globals: Vec<GlobalData>,
}

impl GlobalManager {
    pub(crate) fn new() -> GlobalManager {
        GlobalManager {
            registries: Rc::new(RefCell::new(Vec::new())),
            globals: Vec::new(),
        }
    }

    pub(crate) fn add_global<I, F1, F2>(
        &mut self,
        version: u32,
        implementation: F1,
        filter: Option<F2>,
    ) -> GlobalInner<I>
    where
        I: Interface + AsRef<Resource<I>> + From<Resource<I>>,
        F1: FnMut(Main<I>, u32, DispatchData) + 'static,
        F2: FnMut(ClientInner) -> bool + 'static,
    {
        let implem = RefCell::new(implementation);
        let data = GlobalData {
            version,
            interface: I::NAME,
            destroyed: Rc::new(Cell::new(false)),
            implem: Box::new(move |newid, version, client, data| {
                // insert the object in the map, and call the global bind callback
                // This is done in two times to ensure the client lock is not locked during
                // the callback
                let map = if let Some(ref clientconn) = *client.data.lock().unwrap() {
                    clientconn
                        .map
                        .lock()
                        .unwrap()
                        .insert_at(newid, Object::from_interface::<I>(version, ObjectMeta::new()))?;
                    Some(clientconn.map.clone())
                } else {
                    None
                };
                if let Some(map) = map {
                    (&mut *implem.borrow_mut())(
                        Main::wrap(ResourceInner::from_id(newid, map, client).unwrap()),
                        version,
                        data,
                    )
                }
                Ok(())
            }),
            filter: filter.map(|f| Rc::new(RefCell::new(f)) as Rc<_>),
        };

        let destroyed_marker = data.destroyed.clone();

        let id = self.globals.len() as u32 + 1;

        let filter = data.filter.clone();
        {
            send_new_global(
                &self.registries.borrow(),
                id,
                I::NAME,
                version,
                filter.as_ref().map(|f| &**f),
            );
        }

        self.globals.push(data);

        GlobalInner {
            _i: ::std::marker::PhantomData,
            destroyed_marker,
            id,
            registries: self.registries.clone(),
            filter,
        }
    }

    pub(crate) fn new_registry(&mut self, id: u32, client: ClientInner) {
        let reg = (id, client);
        for (id, global) in self.globals.iter_mut().enumerate() {
            if global.destroyed.get() {
                continue;
            }
            if let Some(ref filter) = global.filter {
                if !(&mut *filter.borrow_mut())(reg.1.clone()) {
                    continue;
                }
            }
            let interface = CString::new(global.interface.as_bytes().to_owned()).unwrap();
            send_global_msg(&reg, id as u32 + 1, interface, global.version);
        }
        self.registries.borrow_mut().push(reg);

        // cleanup destroyed clients, to avoid accumulating stale connections
        self.self_cleanup();
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn bind(
        &self,
        registry_id: u32,
        resource_newid: u32,
        global_id: u32,
        interface: &str,
        version: u32,
        client: ClientInner,
        data: DispatchData,
    ) -> Result<(), ()> {
        if let Some(ref global_data) = self.globals.get((global_id - 1) as usize) {
            if !global_data
                .filter
                .as_ref()
                .map(|f| (&mut *f.borrow_mut())(client.clone()))
                .unwrap_or(true)
            {
                // client is not allowed to see this global
                client.post_error(
                    registry_id,
                    super::display::DISPLAY_ERROR_INVALID_OBJECT,
                    format!("Invalid global {} ({})", interface, global_id),
                );
            } else if global_data.interface != interface {
                client.post_error(
                    registry_id,
                    super::display::DISPLAY_ERROR_INVALID_OBJECT,
                    format!(
                        "Invalid global {} ({}), interface should be {}",
                        interface, global_id, global_data.interface
                    ),
                );
            } else if version == 0 {
                client.post_error(
                    registry_id,
                    super::display::DISPLAY_ERROR_INVALID_OBJECT,
                    format!(
                        "Invalid version for global {} ({}): 0 is not a valid version",
                        interface, global_id
                    ),
                );
            } else if global_data.version < version {
                client.post_error(
                    registry_id,
                    super::display::DISPLAY_ERROR_INVALID_OBJECT,
                    format!(
                        "Invalid version for global {} ({}): have {}, wanted {}",
                        interface, global_id, global_data.version, version
                    ),
                );
            } else {
                // all is good, we insert the object in the map and send it the events
                return (global_data.implem)(resource_newid, version, client, data);
            }
        } else {
            client.post_error(
                registry_id,
                super::display::DISPLAY_ERROR_INVALID_OBJECT,
                format!("Invalid global {} ({})", interface, global_id),
            );
        }
        Ok(())
    }

    fn self_cleanup(&self) {
        self.registries
            .borrow_mut()
            .retain(|&(_, ref client)| client.alive());
    }
}

fn send_global_msg(reg: &(u32, ClientInner), global_id: u32, interface: CString, version: u32) {
    if let Some(ref mut clientconn) = *reg.1.data.lock().unwrap() {
        let _ = clientconn.write_message(&Message {
            sender_id: reg.0,
            opcode: 0,
            args: smallvec![
                Argument::Uint(global_id),
                Argument::Str(Box::new(interface)),
                Argument::Uint(version),
            ],
        });
    }
}

fn send_new_global(
    registries: &[(u32, ClientInner)],
    global_id: u32,
    interface: &str,
    version: u32,
    filter: Option<&RefCell<dyn FnMut(ClientInner) -> bool>>,
) {
    let iface = CString::new(interface.as_bytes().to_owned()).unwrap();
    if let Some(filter) = filter {
        let mut filter = filter.borrow_mut();
        for reg in registries {
            if !(&mut *filter)(reg.1.clone()) {
                continue;
            }
            send_global_msg(reg, global_id, iface.clone(), version)
        }
    } else {
        for reg in registries {
            send_global_msg(reg, global_id, iface.clone(), version)
        }
    }
}

fn send_destroyed_global(
    registries: &[(u32, ClientInner)],
    global_id: u32,
    filter: Option<&RefCell<dyn FnMut(ClientInner) -> bool>>,
) {
    if let Some(filter) = filter {
        let mut filter = filter.borrow_mut();
        for &(id, ref client) in registries {
            if !(&mut *filter)(client.clone()) {
                continue;
            }
            if let Some(ref mut clientconn) = *client.data.lock().unwrap() {
                let _ = clientconn.write_message(&Message {
                    sender_id: id,
                    opcode: 1,
                    args: smallvec![Argument::Uint(global_id)],
                });
            }
        }
    } else {
        for &(id, ref client) in registries {
            if let Some(ref mut clientconn) = *client.data.lock().unwrap() {
                let _ = clientconn.write_message(&Message {
                    sender_id: id,
                    opcode: 1,
                    args: smallvec![Argument::Uint(global_id)],
                });
            }
        }
    }
}
