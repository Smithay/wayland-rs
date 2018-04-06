use std::sync::{Arc, Mutex};

use commons::Interface;
use {NewProxy, Proxy};
use protocol::wl_registry::{self, RequestsTrait};

/// An utility to manage global objects
///
/// This utility provides an implemenation for the registry
/// that track the list of globals for you, as well as utilities
/// to bind them.
pub struct GlobalManager {
    global_list: Arc<Mutex<Vec<(u32, String, u32)>>>,
    registry: Proxy<wl_registry::WlRegistry>,
}

/// An error that occured trying to bind a global
#[derive(Debug)]
pub enum GlobalError {
    /// The requested global was missing
    Missing,
    /// The global abvertized by the server has a lower version number
    /// than the one requested
    VersionTooLow(u32),
}

impl GlobalManager {
    /// Create a global manager handling a registry
    pub fn new(registry: NewProxy<wl_registry::WlRegistry>) -> GlobalManager {
        let global_list = Arc::new(Mutex::new(Vec::new()));
        let global_list_clone = global_list.clone();

        let registry = registry.implement(move |msg, _| match msg {
            wl_registry::Events::Global {
                name,
                interface,
                version,
            } => {
                global_list_clone
                    .lock()
                    .unwrap()
                    .push((name, interface, version));
            }
            wl_registry::Events::GlobalRemove { name } => {
                global_list_clone
                    .lock()
                    .unwrap()
                    .retain(|&(n, _, _)| n != name);
            }
        });

        GlobalManager {
            global_list: global_list,
            registry: registry,
        }
    }

    /// Instanciate a specific global
    ///
    /// This method is only appropriate for globals that are expected to
    /// not exist with multiplicity (sur as `wl_compositor` or `wl_shm`),
    /// as it will only bind a single one.
    pub fn instanciate<I: Interface>(&self, version: u32) -> Result<NewProxy<I>, GlobalError> {
        let list = self.global_list.lock().unwrap();
        for &(id, ref interface, server_version) in &*list {
            if interface == I::NAME {
                if version > server_version {
                    return Err(GlobalError::VersionTooLow(server_version));
                } else {
                    return Ok(self.registry.bind::<I>(version, id).unwrap());
                }
            }
        }
        Err(GlobalError::Missing)
    }

    /// Retrieve the list of currently known globals
    pub fn list(&self) -> Vec<(u32, String, u32)> {
        self.global_list.lock().unwrap().clone()
    }
}
