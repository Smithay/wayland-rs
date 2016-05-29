/// This macro allows you to create a simple wayland environment handler.
///
/// It will define a struct which upon creation communicates with the server
/// to fetch the list of global objects and instanciate them with the newest
/// interface version supported by both the server and the client library.
///
/// This struct comes with a static constructor `init(display)`, which takes
/// a display, and returns the created struct and an `EventIterator` associated
/// with the display and all the global objects.
///
/// Note that none of the events associated with the newly created objects are
/// dispatched (expect for the registry), allowing you to change the event iterators
/// associated with them before dispatching them, if you want to.
///
/// The struct has these public fields:
///
/// - `display`: the `WlDisplay` provided as argument
/// - `registry`: a instance of the `WlRegistry` associated
/// - `globals`: a `Vec` containing the globals advertized by the server, in the format
///   `(global_id, interface_name, version)`
///   Note here that `version` is the version advertized by the server.
/// - One field for each of the objects you specified, of type `Option<T, u32>`, in the format
///   `(proxy, version)`. The value is `None` if this global was not advertized by the server.
///
/// Note that:
///
/// - If you specify several objects of the same interface, only the first one will be
///   populated.
/// - If a global is advertized several times (like `wl_seat` or `wl_output` can be), only
///   the first one will be automatically bound (but all will still be listed in the `globals`
///   list).
///
/// The struct also provides two methods:
///
/// - `fn rebind<T: Proxy>(&self) -> Option<(T, u32)>` which will try to bind once more a global
///   (this allows you to effectively clone a global, and is perfectly legal). It will match
///   the first global of that type that was encountered. Returns `None` if this global type was
///   not encountered.
/// - `fn rebind_id<T: Proxy>(&self, id: u32) -> Option<(T, u32)>` which will try to bind once
///   more a global with given id as listed in `globals`. Returns `None` if given id is not known
///   or if its interface does not match with the provided type.
///
/// Example of use:
///
/// ```no_run
/// # #![allow(dead_code)]
/// #[macro_use] extern crate wayland_client;
///
/// use wayland_client::wayland::get_display;
/// use wayland_client::wayland::compositor::WlCompositor;
/// use wayland_client::wayland::shell::WlShell;
///
/// wayland_env!(WaylandEnv,
///     compositor: WlCompositor,
///     shell: WlShell
/// );
///
/// fn main() {
///     let (display, iter) = get_display().expect("Unable to connect to waylans server.");
///     let (env, iter) = WaylandEnv::init(display, iter);
///     let shell = match env.shell {
///         Some((ref comp, version)) if version >= 2 => comp,
///         _ => panic!("This app requires the wayland interface wl_shell of version >= 2.")
///     };
///     // etc...
/// }
/// ```
#[macro_export]
macro_rules! wayland_env {
    ($structname: ident, $($name: ident : $interface: ty),*) => (
        struct $structname {
            pub display: $crate::wayland::WlDisplay,
            pub registry: $crate::wayland::WlRegistry,
            pub globals: Vec<(u32, String, u32)>,
            $(
                pub $name : Option<($interface, u32)>,
            )*
        }

        impl $structname {
            pub fn init(display: $crate::wayland::WlDisplay, mut iter: $crate::EventIterator) -> ($structname, $crate::EventIterator) {
                use $crate::Event;
                use $crate::wayland::{WaylandProtocolEvent, WlRegistryEvent};

                let registry = display.get_registry();
                match iter.sync_roundtrip() {
                    Ok(_) => {},
                    Err(e) => panic!("Roundtrip with wayland server failed: {:?}", e)
                }

                let mut env = $structname {
                    display: display,
                    registry: registry,
                    globals: Vec::new(),
                    $(
                        $name: None,
                    )*
                };

                while let Ok(Some(evt)) = iter.next_event_dispatch() {
                    match evt {
                        Event::Wayland(WaylandProtocolEvent::WlRegistry(
                            _, WlRegistryEvent::Global(name, interface, version)
                        )) => {
                            env.handle_global(name, interface, version)
                        }
                        _ => {}
                    }
                }

                (env, iter)
            }

            #[allow(dead_code)]
            pub fn rebind<T: $crate::Proxy>(&self) -> Option<(T, u32)> {
                use $crate::Proxy;
                let t_interface = <T as Proxy>::interface_name();
                for &(name, ref interface, version) in &self.globals {
                    if interface == t_interface {
                        let chosen_version = ::std::cmp::min(version, <T as Proxy>::version());
                        let proxy = unsafe { self.registry.bind::<T>(name, chosen_version) };
                        return Some((proxy, chosen_version))
                    }
                }
                return None
            }

            #[allow(dead_code)]
            pub fn rebind_id<T: $crate::Proxy>(&self, id: u32) -> Option<(T, u32)> {
                use $crate::Proxy;
                let t_interface = <T as Proxy>::interface_name();
                for &(name, ref interface, version) in &self.globals {
                    if name == id && interface == t_interface {
                        let chosen_version = ::std::cmp::min(version, <T as Proxy>::version());
                        let proxy = unsafe { self.registry.bind::<T>(name, chosen_version) };
                        return Some((proxy, chosen_version))
                    }
                }
                return None
            }

            fn handle_global(&mut self, name: u32, interface: String, version: u32) {
                use $crate::Proxy;
                match interface {
                    $(
                        ref s if &s[..] == <$interface as Proxy>::interface_name() => {
                            let chosen_version = ::std::cmp::min(version, <$interface as Proxy>::version());
                            if self.$name.is_none() {
                                let proxy = unsafe { self.registry.bind::<$interface>(name, chosen_version) };
                                self.$name = Some((proxy, chosen_version));
                            }
                        }
                    )*
                    _ => {}
                }
                self.globals.push((name, interface, version));
            }
        }
    )
}