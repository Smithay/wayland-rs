use std::cell::RefCell;
use std::os::raw::{c_int, c_void};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Weak};

use crate::{Interface, Main, Proxy, RawEvent};
use wayland_commons::filter::Filter;
use wayland_commons::user_data::UserData;
use wayland_commons::wire::ArgumentType;
use wayland_commons::MessageGroup;

use super::EventQueueInner;

use wayland_sys::client::*;
use wayland_sys::common::*;

pub struct ProxyInternal {
    alive: AtomicBool,
    user_data: UserData,
}

impl ProxyInternal {
    pub fn new(user_data: UserData) -> ProxyInternal {
        ProxyInternal { alive: AtomicBool::new(true), user_data }
    }
}

pub(crate) struct ProxyInner {
    internal: Option<Arc<ProxyInternal>>,
    ptr: *mut wl_proxy,
    wrapping: Option<*mut wl_proxy>,
    pub(crate) display: Option<Weak<super::display::DisplayGuard>>,
}

unsafe impl Send for ProxyInner {}
unsafe impl Sync for ProxyInner {}

impl ProxyInner {
    pub(crate) fn is_alive(&self) -> bool {
        if let Some(ref weak) = self.display {
            if Weak::strong_count(weak) == 0 {
                // the connection is dead
                return false;
            }
        }

        self.internal.as_ref().map(|i| i.alive.load(Ordering::Acquire)).unwrap_or(true)
    }

    pub(crate) fn is_external(&self) -> bool {
        self.internal.is_none()
    }

    pub(crate) fn version(&self) -> u32 {
        if !self.is_alive() {
            return 0;
        }
        let version =
            unsafe { ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_get_version, self.ptr) as u32 };
        if version == 0 {
            // For backcompat reasons the C libs return 0 as a version for the wl_display
            // So override it
            return 1;
        }
        version
    }

    pub(crate) fn is_interface<I: Interface>(&self) -> bool {
        if !self.is_alive() {
            return false;
        }
        unsafe {
            let c_class_ptr = ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_get_class, self.ptr);
            let class = std::ffi::CStr::from_ptr(c_class_ptr);
            let i_class = std::ffi::CStr::from_ptr((*I::c_interface()).name);
            class == i_class
        }
    }

    pub(crate) fn id(&self) -> u32 {
        if !self.is_alive() {
            return 0;
        }
        unsafe { ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_get_id, self.ptr) }
    }

    pub(crate) fn user_data(&self) -> &UserData {
        static INVALID_USERDATA: UserData = UserData::new();
        if let Some(ref inner) = self.internal {
            &inner.user_data
        } else {
            INVALID_USERDATA.set_threadsafe(|| ());
            &INVALID_USERDATA
        }
    }

    pub(crate) fn send<I, J>(&self, msg: I::Request, version: Option<u32>) -> Option<ProxyInner>
    where
        I: Interface + AsRef<Proxy<I>> + From<Proxy<I>>,
        J: Interface + AsRef<Proxy<J>> + From<Proxy<J>>,
    {
        let destructor = msg.is_destructor();
        let opcode = msg.opcode();

        // figure out if the call creates an object
        let nid_idx = I::Request::MESSAGES[opcode as usize]
            .signature
            .iter()
            .position(|&t| t == ArgumentType::NewId);

        let alive = self.is_alive();

        let ret = if let Some(mut nid_idx) = nid_idx {
            if let Some(o) = I::Request::child(opcode, 1, &()) {
                if !o.is_interface::<J>() {
                    panic!("Trying to use 'send_constructor' with the wrong return type. Required interface {} but the message creates interface {}", J::NAME, o.interface)
                }
            } else {
                // there is no target interface in the protocol, this is a generic object-creating
                // function (likely wl_registry.bind), the newid arg will thus expand to (str, u32, obj)
                nid_idx += 2;
            }

            let version = version.unwrap_or_else(|| self.version());

            if alive {
                if self.wrapping.is_none() {
                    panic!("Attemping to create an object from a non-attached proxy.");
                }
                unsafe {
                    let ptr = msg.as_raw_c_in(|opcode, args| {
                        assert!(
                            args[nid_idx].o.is_null(),
                            "Trying to use 'send_constructor' with a non-placeholder object."
                        );
                        ffi_dispatch!(
                            WAYLAND_CLIENT_HANDLE,
                            wl_proxy_marshal_array_constructor_versioned,
                            self.wrapping.unwrap_or(self.ptr),
                            opcode,
                            args.as_mut_ptr(),
                            J::c_interface(),
                            version
                        )
                    });
                    let mut new_proxy = ProxyInner::init_from_c_ptr::<J>(ptr);
                    new_proxy.display = self.display.clone();
                    Some(new_proxy)
                }
            } else {
                // Create a dead proxy ex-nihilo
                Some(ProxyInner::dead())
            }
        } else {
            // not a constructor, nothing much to do
            if alive {
                msg.as_raw_c_in(|opcode, args| unsafe {
                    ffi_dispatch!(
                        WAYLAND_CLIENT_HANDLE,
                        wl_proxy_marshal_array,
                        self.wrapping.unwrap_or(self.ptr),
                        opcode,
                        args.as_ptr() as *mut _
                    );
                });
            }
            None
        };

        if destructor && alive {
            // we need to destroy the proxy now
            if let Some(ref internal) = self.internal {
                internal.alive.store(false, Ordering::Release);
                unsafe {
                    let user_data =
                        ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_get_user_data, self.ptr);
                    ffi_dispatch!(
                        WAYLAND_CLIENT_HANDLE,
                        wl_proxy_set_user_data,
                        self.ptr,
                        std::ptr::null_mut()
                    );
                    let _ = Box::from_raw(user_data as *mut ProxyUserData<I>);
                }
            }
            unsafe {
                ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_destroy, self.ptr);
            }
        }

        ret
    }

    pub(crate) fn equals(&self, other: &ProxyInner) -> bool {
        if !self.is_alive() {
            return false;
        }
        match (&self.internal, &other.internal) {
            (&Some(ref my_inner), &Some(ref other_inner)) => Arc::ptr_eq(my_inner, other_inner),
            (&None, &None) => self.ptr == other.ptr,
            _ => false,
        }
    }

    pub(crate) fn detach(&mut self) {
        if !self.is_external() && !self.is_alive() {
            return;
        }

        if let Some(ptr) = self.wrapping.take() {
            // self.ptr == self.wrapping means we are a Main<_>
            if ptr != self.ptr {
                unsafe {
                    ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_wrapper_destroy, ptr);
                }
            }
        }
    }

    pub(crate) fn attach(&mut self, queue: &EventQueueInner) {
        if !self.is_external() && !self.is_alive() {
            return;
        }

        let wrapper_ptr;
        unsafe {
            wrapper_ptr = ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_create_wrapper, self.ptr);
            queue.assign_proxy(wrapper_ptr);
        }

        self.wrapping = Some(wrapper_ptr);
    }

    pub(crate) fn c_ptr(&self) -> *mut wl_proxy {
        self.wrapping.unwrap_or(self.ptr)
    }

    pub fn assign<I, E>(&self, filter: Filter<E>)
    where
        I: Interface + AsRef<Proxy<I>> + From<Proxy<I>> + Sync,
        E: From<(Main<I>, I::Event)> + 'static,
        I::Event: MessageGroup<Map = super::ProxyMap>,
    {
        if self.is_external() {
            panic!("Cannot assign an external proxy to a filter.");
        }

        if !self.is_alive() {
            return;
        }

        unsafe {
            let user_data = ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_get_user_data, self.ptr)
                as *mut ProxyUserData<I>;
            if let Ok(ref mut guard) = (*user_data).implem.try_borrow_mut() {
                **guard =
                    Some(Box::new(move |evt, obj, data| filter.send((obj, evt).into(), data)));
            } else {
                panic!("Re-assigning an object from within its own callback is not supported.");
            }
        }
    }

    pub(crate) unsafe fn init_from_c_ptr<I: Interface + From<Proxy<I>> + AsRef<Proxy<I>>>(
        ptr: *mut wl_proxy,
    ) -> Self {
        let new_user_data = Box::new(ProxyUserData::<I>::new(UserData::new()));
        let internal = new_user_data.internal.clone();

        ffi_dispatch!(
            WAYLAND_CLIENT_HANDLE,
            wl_proxy_add_dispatcher,
            ptr,
            proxy_dispatcher::<I>,
            &::wayland_sys::RUST_MANAGED as *const _ as *const _,
            Box::into_raw(new_user_data) as *mut _
        );

        // We are a Main<_>, so ptr == wrapping
        ProxyInner { internal: Some(internal), ptr, wrapping: Some(ptr), display: None }
    }

    fn dead() -> Self {
        ProxyInner {
            internal: Some(Arc::new(ProxyInternal {
                alive: AtomicBool::new(false),
                user_data: UserData::new(),
            })),
            ptr: std::ptr::null_mut(),
            wrapping: None,
            display: None,
        }
    }

    pub(crate) unsafe fn from_c_ptr<I: Interface + From<Proxy<I>> + AsRef<Proxy<I>>>(
        ptr: *mut wl_proxy,
    ) -> Self {
        if ptr.is_null() {
            return Self::dead();
        }

        let is_managed = {
            ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_get_listener, ptr)
                == &::wayland_sys::RUST_MANAGED as *const u8 as *const _
        };
        let internal = if is_managed {
            let user_data = ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_get_user_data, ptr)
                as *mut ProxyUserData<I>;
            Some((*user_data).internal.clone())
        } else {
            None
        };
        ProxyInner { internal, ptr, wrapping: None, display: None }
    }

    pub(crate) unsafe fn from_external_display(d: *mut wl_proxy) -> ProxyInner {
        ProxyInner { internal: None, ptr: d, wrapping: None, display: None }
    }
}

impl Clone for ProxyInner {
    fn clone(&self) -> ProxyInner {
        let mut new = ProxyInner {
            internal: self.internal.clone(),
            ptr: self.ptr,
            wrapping: None,
            display: self.display.clone(),
        };
        if !self.is_external() && !self.is_alive() {
            return new;
        }
        if let Some(ptr) = self.wrapping {
            if ptr != self.ptr {
                // create a new wrapper to keep correct track of them
                let wrapper_ptr;
                unsafe {
                    wrapper_ptr =
                        ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_create_wrapper, ptr);
                }
                new.wrapping = Some(wrapper_ptr);
            } else {
                // self.wrapping == self.ptr means we are a Main<_>, not a wrapper
                new.wrapping = Some(self.ptr);
            }
        }
        new
    }
}

impl Drop for ProxyInner {
    fn drop(&mut self) {
        // always detach on drop to avoid leaking wrappers
        self.detach();
    }
}

type BoxedCallback<I> = Box<dyn Fn(<I as Interface>::Event, Main<I>, crate::DispatchData<'_>)>;

struct ProxyUserData<I: Interface + From<Proxy<I>> + AsRef<Proxy<I>>> {
    internal: Arc<ProxyInternal>,
    implem: RefCell<Option<BoxedCallback<I>>>,
}

impl<I: Interface + From<Proxy<I>> + AsRef<Proxy<I>>> ProxyUserData<I> {
    fn new(user_data: UserData) -> ProxyUserData<I> {
        ProxyUserData {
            internal: Arc::new(ProxyInternal::new(user_data)),
            implem: RefCell::new(None),
        }
    }
}

unsafe extern "C" fn proxy_dispatcher<I: Interface>(
    _implem: *const c_void,
    proxy: *mut c_void,
    opcode: u32,
    _msg: *const wl_message,
    args: *const wl_argument,
) -> c_int
where
    I: Interface + From<Proxy<I>> + AsRef<Proxy<I>>,
{
    let proxy = proxy as *mut wl_proxy;

    // We don't need to worry about panic-safeness, because if there is a panic,
    // we'll abort the process, so no access to corrupted data is possible.
    let ret = ::std::panic::catch_unwind(move || {
        let must_destroy = I::Event::MESSAGES[opcode as usize].destructor;
        // retrieve the impl
        let user_data = ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_get_user_data, proxy);
        {
            // a scope to ensure the &mut reference to the user data no longer exists when
            // the callback is invoked
            let (implem, internal) = {
                let user_data = &mut *(user_data as *mut ProxyUserData<I>);

                if must_destroy {
                    user_data.internal.alive.store(false, Ordering::Release);
                    ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_destroy, proxy);
                }
                // remove the implem from the user data, so that it remains valid even if the
                // object is destroyed from within the callback
                (user_data.implem.borrow_mut().take(), user_data.internal.clone())
            };
            // if there is an implem, call it, otherwise call the fallback
            match implem {
                Some(ref implem) => {
                    // parse the message:
                    let msg = I::Event::from_raw_c(proxy as *mut _, opcode, args)?;
                    // create the proxy object
                    let mut proxy_inner = ProxyInner::from_c_ptr::<I>(proxy);
                    // This proxy must be a Main, so it as attached wrapping itself
                    proxy_inner.wrapping = Some(proxy_inner.ptr);
                    let proxy_obj = crate::Main::wrap(proxy_inner);
                    super::event_queue::DISPATCH_METADATA.with(|meta| {
                        let mut meta = meta.borrow_mut();
                        let (_, ref mut dispatch_data) = *meta;
                        implem(msg, proxy_obj, dispatch_data.reborrow());
                    })
                }
                None => {
                    // parse the message:
                    let msg = parse_raw_event::<I>(opcode, args);
                    // create the proxy object
                    let proxy_obj = crate::Main::wrap(ProxyInner::from_c_ptr::<I>(proxy));
                    super::event_queue::DISPATCH_METADATA.with(|meta| {
                        let mut meta = meta.borrow_mut();
                        let (ref mut fallback, ref mut dispatch_data) = *meta;
                        (&mut *fallback)(msg, proxy_obj, dispatch_data.reborrow());
                    })
                }
            }

            // if the proxy is still alive, put the implem back in place
            if internal.alive.load(Ordering::Acquire) {
                let user_data = &mut *(user_data as *mut ProxyUserData<I>);
                let mut implem_place = user_data.implem.borrow_mut();
                // only place the implem back if it has not been replaced with an other
                if implem_place.is_none() {
                    *implem_place = implem;
                }
            }
        }
        if must_destroy {
            // final cleanup
            let _ = Box::from_raw(user_data as *mut ProxyUserData<I>);
        }
        Ok(())
    });
    // check the return status
    match ret {
        Ok(Ok(())) => 0,
        Ok(Err(())) => {
            eprintln!(
                "[wayland-client error] Attempted to dispatch unknown opcode {} for {}, aborting.",
                opcode,
                I::NAME
            );
            libc::abort();
        }
        Err(_) => {
            eprintln!("[wayland-client error] A handler for {} panicked.", I::NAME);
            libc::abort()
        }
    }
}

unsafe fn parse_raw_event<I: Interface>(opcode: u32, args: *const wl_argument) -> RawEvent {
    let desc = &I::Event::MESSAGES[opcode as usize];

    let c_args = std::slice::from_raw_parts(args, desc.signature.len());

    let mut args = Vec::with_capacity(c_args.len());
    // parse the arguments one by one
    for (t, a) in desc.signature.iter().zip(c_args.iter()) {
        args.push(match t {
            ArgumentType::Int => crate::Argument::Int(a.i),
            ArgumentType::Uint => crate::Argument::Uint(a.u),
            ArgumentType::Fixed => crate::Argument::Float((a.f as f32) / 256.),
            ArgumentType::Array => {
                if a.a.is_null() {
                    crate::Argument::Array(None)
                } else {
                    let array = &*a.a;
                    crate::Argument::Array(Some(
                        ::std::slice::from_raw_parts(array.data as *const u8, array.size)
                            .to_owned(),
                    ))
                }
            }
            ArgumentType::Str => {
                if a.s.is_null() {
                    crate::Argument::Str(None)
                } else {
                    crate::Argument::Str(Some(
                        ::std::ffi::CStr::from_ptr(a.s).to_string_lossy().into_owned(),
                    ))
                }
            }
            ArgumentType::Fd => crate::Argument::Fd(a.h),
            ArgumentType::Object => {
                if a.o.is_null() {
                    crate::Argument::Object(None)
                } else {
                    crate::Argument::Object(Some(Proxy::from_c_ptr(a.o as *mut _)))
                }
            }
            ArgumentType::NewId => {
                if a.o.is_null() {
                    crate::Argument::NewId(None)
                } else {
                    crate::Argument::NewId(Some(Main::from_c_ptr(a.o as *mut _)))
                }
            }
        });
    }

    RawEvent { interface: I::NAME, opcode: opcode as u16, name: desc.name, args }
}
