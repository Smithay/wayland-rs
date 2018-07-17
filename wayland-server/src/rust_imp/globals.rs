use std::cell::{Cell, RefCell};
use std::ffi::CString;
use std::rc::Rc;

use wayland_commons::map::Object;
use wayland_commons::wire::{Argument, Message};

use {Implementation, Interface, NewResource};

use super::resources::ObjectMeta;
use super::{ClientInner, EventLoopInner, NewResourceInner};

pub(crate) struct GlobalInner<I: Interface> {
    _i: ::std::marker::PhantomData<*const I>,
    destroyed_marker: Rc<Cell<bool>>,
    id: u32,
    registries: Rc<RefCell<Vec<(u32, ClientInner)>>>,
}

impl<I: Interface> GlobalInner<I> {
    pub fn destroy(self) {
        self.destroyed_marker.set(true);
        send_destroyed_global(&self.registries.borrow(), self.id);
    }
}

struct GlobalData {
    version: u32,
    interface: &'static str,
    destroyed: Rc<Cell<bool>>,
    implem: Box<Fn(u32, u32, ClientInner) -> Result<(), ()>>,
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

    pub(crate) fn add_global<I: Interface, Impl>(
        &mut self,
        event_loop: &EventLoopInner,
        version: u32,
        implementation: Impl,
    ) -> GlobalInner<I>
    where
        Impl: Implementation<NewResource<I>, u32> + 'static,
    {
        let implem = RefCell::new(implementation);
        let data = GlobalData {
            version,
            interface: I::NAME,
            destroyed: Rc::new(Cell::new(false)),
            implem: Box::new(move |newid, version, client| {
                // insert the object in the map, and call the global bind callback
                if let Some(ref clientconn) = *client.data.lock().unwrap() {
                    clientconn
                        .map
                        .lock()
                        .unwrap()
                        .insert_at(newid, Object::from_interface::<I>(version, ObjectMeta::new()))?;
                    implem.borrow_mut().receive(
                        version,
                        NewResource::wrap(
                            NewResourceInner::from_id(newid, clientconn.map.clone(), client.clone()).unwrap(),
                        ),
                    )
                }
                Ok(())
            }),
        };

        let destroyed_marker = data.destroyed.clone();

        self.globals.push(data);
        let id = self.globals.len() as u32;

        send_new_global(&self.registries.borrow(), id, I::NAME, version);

        GlobalInner {
            _i: ::std::marker::PhantomData,
            destroyed_marker,
            id,
            registries: self.registries.clone(),
        }
    }

    pub(crate) fn new_registry(&mut self, id: u32, client: ClientInner) {
        let reg = (id, client);
        for (id, global) in self.globals.iter().enumerate() {
            if global.destroyed.get() {
                continue;
            }
            let interface = CString::new(global.interface.as_bytes().to_owned()).unwrap();
            send_global_msg(&reg, id as u32, interface, global.version);
        }
        self.registries.borrow_mut().push(reg);

        // cleanup destroyed clients, to avoid accumulating stale connections
        self.self_cleanup();
    }

    pub(crate) fn bind(
        &self,
        registry_id: u32,
        resource_newid: u32,
        global_id: u32,
        interface: &str,
        version: u32,
        client: ClientInner,
    ) -> Result<(), ()> {
        if let Some(ref global_data) = self.globals.get(global_id as usize) {
            if global_data.interface != interface {
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
                (global_data.implem)(resource_newid, version, client);
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
    if let Some(mut clientconn) = reg.1.data.lock().unwrap().take() {
        let _ = clientconn.write_message(&Message {
            sender_id: reg.0,
            opcode: 0,
            args: vec![
                Argument::Uint(global_id),
                Argument::Str(interface),
                Argument::Uint(version),
            ],
        });
    }
}

fn send_new_global(registries: &[(u32, ClientInner)], global_id: u32, interface: &str, version: u32) {
    let iface = CString::new(interface.as_bytes().to_owned()).unwrap();
    for reg in registries {
        send_global_msg(reg, global_id, iface.clone(), version)
    }
}

fn send_destroyed_global(registries: &[(u32, ClientInner)], global_id: u32) {
    for &(id, ref client) in registries {
        if let Some(mut clientconn) = client.data.lock().unwrap().take() {
            let _ = clientconn.write_message(&Message {
                sender_id: id,
                opcode: 1,
                args: vec![Argument::Uint(global_id)],
            });
        }
    }
}
