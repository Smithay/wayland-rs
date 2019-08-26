use std::ops::Deref;

use super::AnonymousObject;
use wayland_commons::user_data::UserData;
use wayland_commons::Interface;

use wayland_sys::client::*;

use crate::event_queue::QueueToken;

use crate::imp::ProxyInner;

use wayland_commons::{filter::Filter, MessageGroup};

/// An handle to a wayland proxy
///
/// This represents a wayland object instantiated in your client
/// session. Several handles to the same object can exist at a given
/// time, and cloning them won't create a new protocol object, only
/// clone the handle. The lifetime of the protocol object is **not**
/// tied to the lifetime of these handles, but rather to sending or
/// receiving destroying messages.
///
/// These handles are notably used to send requests to the server. To do this
/// you need to convert them to the corresponding Rust object (using `.into()`)
/// and use methods on the Rust object.
///
/// This handle is the most conservative one: it can be sent between threads,
/// but you cannot send any message that would create a new object using it.
/// You must attach it to a event queue, that will host the newly created objects.
pub struct Proxy<I: Interface> {
    _i: ::std::marker::PhantomData<&'static I>,
    pub(crate) inner: ProxyInner,
}

impl<I: Interface> Clone for Proxy<I> {
    fn clone(&self) -> Proxy<I> {
        let cloned = self.inner.clone();
        // an owned Proxy must always be detached
        cloned.detach();
        Proxy {
            _i: ::std::marker::PhantomData,
            inner: cloned,
        }
    }
}

impl<I: Interface> PartialEq for Proxy<I> {
    fn eq(&self, other: &Proxy<I>) -> bool {
        self.equals(other)
    }
}

impl<I: Interface> Eq for Proxy<I> {}

impl<I: Interface> Proxy<I> {
    pub(crate) fn wrap(inner: ProxyInner) -> Proxy<I> {
        Proxy {
            _i: ::std::marker::PhantomData,
            inner,
        }
    }

    /// Send a request creating an object through this object
    ///
    /// **Warning:** This method is mostly intended to be used by code generated
    /// by `wayland-scanner`, and you should probably never need to use it directly,
    /// but rather use the appropriate methods on the Rust object.
    ///
    /// This is the generic method to send requests that create objects
    ///
    /// The slot in the message corresponding with the newly created object must have
    /// been filled by a placeholder object (see `child_placeholder`).
    pub fn send<J>(&self, msg: I::Request, version: Option<u32>) -> Result<Option<MainProxy<J>>, ()>
    where
        J: Interface,
    {
        if !self.is_alive() {
            return Err(());
        }
        if msg.since() > self.version() {
            let opcode = msg.opcode() as usize;
            panic!(
                "Cannot send request {} which requires version >= {} on proxy {}@{} which is version {}.",
                I::Request::MESSAGES[opcode].name,
                msg.since(),
                I::NAME,
                self.id(),
                self.version()
            );
        }
        self.inner
            .send::<I, J>(msg, version)
            .map(|o| o.map(MainProxy::wrap))
    }

    /// Check if the object associated with this proxy is still alive
    ///
    /// Will return `false` if the object has been destroyed.
    ///
    /// If the object is not managed by this library, this will always
    /// returns `true`.
    pub fn is_alive(&self) -> bool {
        self.inner.is_alive()
    }

    /// Retrieve the interface version of this wayland object instance
    ///
    /// Returns 0 on dead objects
    pub fn version(&self) -> u32 {
        self.inner.version()
    }

    /// Retrieve the object id of this wayland object
    pub fn id(&self) -> u32 {
        self.inner.id()
    }

    /// Access the UserData associated to this object
    ///
    /// Each wayland object has an associated UserData, that can store
    /// a payload of arbitrary type and is shared by all proxies of this
    /// object.
    ///
    /// See UserData documentation for more details.
    pub fn user_data(&self) -> &UserData {
        self.inner.user_data()
    }

    /// Check if the other proxy refers to the same underlying wayland object
    pub fn equals(&self, other: &Proxy<I>) -> bool {
        self.inner.equals(&other.inner)
    }

    /// Attach this proxy to the event queue represented by this token
    ///
    /// Once a proxy is attached, you can use it to send requests that
    /// create new objects. These new objects will be handled by the
    /// event queue represented by the provided token.
    ///
    /// This does not impact the events received by this object, which
    /// are still handled by their original event queue.
    pub fn attach(self, token: QueueToken) -> AttachedProxy<I> {
        self.inner.attach(&token.inner);
        AttachedProxy {
            inner: self,
            _s: std::marker::PhantomData,
        }
    }

    /// Erase the actual type of this proxy
    pub fn anonymize(self) -> Proxy<AnonymousObject> {
        Proxy {
            _i: ::std::marker::PhantomData,
            inner: self.inner,
        }
    }
}

/// A handle to a proxy that has been attached to an event queue
///
/// As opposed to a mere `Proxy`, you can use it to send requests
/// that create new objects. The created objects will be handled
/// by the event queue this proxy has been atatched to.
pub struct AttachedProxy<I: Interface> {
    // AttachedProxy is *not* send/sync
    _s: ::std::marker::PhantomData<*mut ()>,
    inner: Proxy<I>,
}

impl<I: Interface> AttachedProxy<I> {
    /// Detach this handle, converting it into a threadsafe Proxy
    pub fn detach(self) -> Proxy<I> {
        self.inner.inner.detach();
        self.inner
    }
}

impl<I: Interface> Deref for AttachedProxy<I> {
    type Target = Proxy<I>;

    fn deref(&self) -> &Proxy<I> {
        &self.inner
    }
}

/// A main handle to a proxy
pub struct MainProxy<I: Interface> {
    inner: AttachedProxy<I>,
}

impl<I: Interface> MainProxy<I> {
    pub(crate) fn wrap(inner: ProxyInner) -> MainProxy<I> {
        MainProxy {
            inner: AttachedProxy {
                inner: Proxy {
                    _i: std::marker::PhantomData,
                    inner,
                },
                _s: std::marker::PhantomData,
            },
        }
    }

    pub fn assign<E>(&self, filter: Filter<E>)
    where
        E: From<(MainProxy<I>, I::Event)> + 'static,
        I::Event: MessageGroup<Map = crate::ProxyMap>,
    {
        self.inner.inner.inner.assign(filter);
    }
}

impl<I: Interface> Deref for MainProxy<I> {
    type Target = AttachedProxy<I>;

    fn deref(&self) -> &AttachedProxy<I> {
        &self.inner
    }
}

/*
 * C-interfacing stuff
 */

impl<I: Interface> MainProxy<I> {
    /// Create a `MainProxy` instance from a C pointer
    ///
    /// Create a `MainProxy` from a raw pointer to a wayland object from the
    /// C library.
    ///
    /// This will take control of the underlying proxy & manage it. To be safe
    /// you must ensure that:
    ///
    /// - The provided proxy has not already been used in any way (it was just created)
    /// - This is called from the same thread as the one hosting the event queue
    ///   handling this proxy
    ///
    /// In order to handle protocol races, invoking it with a NULL pointer will
    /// create an already-dead object.
    ///
    /// NOTE: This method will panic if called while the `native_lib` feature is
    /// not activated.
    pub unsafe fn from_c_ptr(_ptr: *mut wl_proxy) -> MainProxy<I>
    where
        I: From<Proxy<I>>,
    {
        #[cfg(feature = "native_lib")]
        {
            Proxy {
                _i: ::std::marker::PhantomData,
                inner: ProxyInner::init_from_c_ptr::<I>(_ptr),
            }
        }
        #[cfg(not(feature = "native_lib"))]
        {
            panic!("[wayland-client] C interfacing methods can only be used with the `native_lib` cargo feature.")
        }
    }
}

impl<I: Interface> Proxy<I> {
    /// Check whether this proxy is managed by the library or not
    ///
    /// See `from_c_ptr` for details.
    ///
    /// NOTE: This method will panic if called while the `native_lib` feature is
    /// not activated.
    pub fn is_external(&self) -> bool {
        #[cfg(feature = "native_lib")]
        {
            self.inner.is_external()
        }
        #[cfg(not(feature = "native_lib"))]
        {
            panic!("[wayland-client] C interfacing methods can only be used with the `native_lib` cargo feature.")
        }
    }

    /// Get a raw pointer to the underlying wayland object
    ///
    /// Retrieve a pointer to the object from the `libwayland-client.so` library.
    /// You will mostly need it to interface with C libraries needing access
    /// to wayland objects (to initialize an opengl context for example).
    ///
    /// NOTE: This method will panic if called while the `native_lib` feature is
    /// not activated.
    pub fn c_ptr(&self) -> *mut wl_proxy {
        #[cfg(feature = "native_lib")]
        {
            self.inner.c_ptr()
        }
        #[cfg(not(feature = "native_lib"))]
        {
            panic!("[wayland-client] C interfacing methods can only be used with the `native_lib` cargo feature.")
        }
    }

    /// Create a `Proxy` instance from a C pointer
    ///
    /// Create a `Proxy` from a raw pointer to a wayland object from the
    /// C library.
    ///
    /// If the pointer was previously obtained by the `c_ptr()` method, this
    /// constructs a new proxy for the same object just like the `clone()`
    /// method would have.
    ///
    /// If the object was created by some other C library you are interfacing
    /// with, it will be created in an "unmanaged" state: wayland-client will
    /// treat it as foreign, and as such most of the safeties will be absent.
    /// Notably the lifetime of the object can't be tracked, so the `alive()`
    /// method will always return `false` and you are responsible of not using
    /// an object past its destruction (as this would cause a protocol error).
    /// You will also be unable to associate any user data pointer to this object.
    ///
    /// In order to handle protocol races, invoking it with a NULL pointer will
    /// create an already-dead object.
    ///
    /// NOTE: This method will panic if called while the `native_lib` feature is
    /// not activated.
    pub unsafe fn from_c_ptr(_ptr: *mut wl_proxy) -> Proxy<I>
    where
        I: From<Proxy<I>>,
    {
        #[cfg(feature = "native_lib")]
        {
            Proxy {
                _i: ::std::marker::PhantomData,
                inner: ProxyInner::from_c_ptr::<I>(_ptr),
            }
        }
        #[cfg(not(feature = "native_lib"))]
        {
            panic!("[wayland-client] C interfacing methods can only be used with the `native_lib` cargo feature.")
        }
    }
}

#[cfg(feature = "native_lib")]
impl Proxy<::protocol::wl_display::WlDisplay> {
    pub(crate) unsafe fn from_c_display_wrapper(
        ptr: *mut wl_proxy,
    ) -> Proxy<::protocol::wl_display::WlDisplay> {
        Proxy {
            _i: ::std::marker::PhantomData,
            inner: ProxyInner::from_c_display_wrapper(ptr),
        }
    }
}
