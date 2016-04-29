use wayland_sys::server::*;

use std::os::raw::c_void;

use client::wrap_client;
use display::Display;
use requests::{ResourceParent, RequestIterator, IteratorDispatch};
use Resource;

pub unsafe trait GlobalResource: Resource {
    type Creator: GlobalCreator;
}

/// All possible wayland global instances
///
/// This enum does a first sorting of request, with a variant for each
/// protocol extension activated in cargo features.
///
/// As the number of variant of this enum can change depending on which cargo features are
/// activated, it should *never* be matched exhaustively. It contains an hidden, never-used
/// variant to ensure it.
///
/// However, you should always match it for *all* the globals that have previously
/// been advertized, in order to respect the wayland protocol.
pub enum GlobalInstance {
    Wayland(::wayland::WaylandProtocolGlobalInstance),
    #[cfg(all(feature = "unstable-protocols", feature="wpu-xdg_shell"))]
    XdgShellUnstableV5(::xdg_shell::XdgShellUnstableV5ProtocolGlobalInstance),
    #[doc(hidden)]
    __DoNotMatchThis,
}

pub trait GlobalCreator {
    fn new() -> Box<GlobalCreator> where Self: Sized;
    unsafe fn create_instance(&self, client: *mut wl_client, version: u32, id: u32, reqiter: &RequestIterator) -> Result<GlobalInstance, ()>;
}

pub unsafe fn create_instance<R: GlobalResource>(client: *mut wl_client, version: u32, id: u32, reqiter: &RequestIterator) -> Result<R, ()> {
    if version <= R::max_version() {
        let res = ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_resource_create,
             client, R::interface(), version as i32, id);
        let mut res = R::from_ptr(res);
        res.set_req_iterator(reqiter);
        Ok(res)
    } else {
        Err(())
    }
}

#[derive(Copy,Clone,PartialEq,Eq,Debug,Hash)]
pub struct GlobalId { id: usize }

pub struct Global<R: GlobalResource> {
    _r: ::std::marker::PhantomData<fn() -> R>,
    ptr: *mut wl_global,
    userdata: Box<(Box<IteratorDispatch>,Box<GlobalCreator>)>
}

impl<R: GlobalResource> Drop for Global<R> {
    fn drop(&mut self) {
        unsafe { ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_global_destroy, self.ptr) }
    }
}

impl<R: GlobalResource> Global<R> {
    pub fn id(&self) -> GlobalId {
        GlobalId { id: &*self.userdata as *const (_,_) as usize }
    }
}

pub fn declare_global<R, I>(display: &Display, iter_disp: I) -> Global<R> where
    R: GlobalResource,
    I: IteratorDispatch + Send + Sync + 'static
{
    let iter_disp = Box::new(iter_disp) as Box<IteratorDispatch>;

    let mut userdata = Box::new((iter_disp,R::Creator::new()));

    let global = unsafe {
        let ptr = ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_global_create,
            display.ptr(), R::interface(), R::max_version() as i32,
            &mut *userdata as *mut (_, _) as *mut _, global_bind);
        Global {
             _r: ::std::marker::PhantomData,
             ptr: ptr,
             userdata: userdata
        }
    };

    global
}

extern "C" fn global_bind(client: *mut wl_client, userdata: *mut c_void, version: u32, id: u32) {
    let userdata = unsafe { &*(userdata as *const (Box<IteratorDispatch>,Box<GlobalCreator>)) };
    let global_id = GlobalId { id: userdata as *const _ as usize };
    let client_id = wrap_client(client);
    let reqiter = userdata.0.get_iterator(client_id, ResourceParent::Global(global_id));
    let instance = unsafe { userdata.1.create_instance(client, version, id, reqiter) };
    match instance {
        Ok(glob_inst) => {
            // TODO : forward instance to user
        },
        Err(()) => {
            // This is a client protocol error, report it
        }
    }
}

#[macro_escape]
macro_rules! protocol_globals (
    ($pvariant: ident, $pname: ident, $($gname: ident => $gty: ty),*) => (
        pub enum $pname {
            $(
                $gname($gty)
            ),*
        }

        $(
            unsafe impl ::globals::GlobalResource for $gty {
                type Creator = global_impls::$gname;
            }
        )*

        #[doc(hidden)]
        pub mod global_impls {
            use ::requests::RequestIterator;
            use ::globals::{GlobalCreator, GlobalInstance, create_instance};
            use wayland_sys::server::wl_client;
            use super::$pname;
            $(
                pub struct $gname;
                impl GlobalCreator for $gname {
                    fn new() -> Box<GlobalCreator> {
                        Box::new($gname) as Box<GlobalCreator>
                    }

                    unsafe fn create_instance(&self, client: *mut wl_client, version: u32, id: u32, reqiter: &RequestIterator)
                        -> Result<GlobalInstance, ()> {
			create_instance::<$gty>(client, version, id, reqiter).map(|r|
                            GlobalInstance::$pvariant($pname::$gname(r))
                        )
                    }
                }
            )*
        }
    )
);
