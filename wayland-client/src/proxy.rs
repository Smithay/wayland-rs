use wayland_commons::utils::UserData;
use wayland_commons::{AnonymousObject, Interface};

#[cfg(feature = "native_lib")]
use wayland_sys::client::*;

use event_queue::QueueToken;

use imp::{NewProxyInner, ProxyInner};

use wayland_commons::MessageGroup;
use ProxyMap;

/// An handle to a wayland proxy
///
/// This represents a wayland object instantiated in your client
/// session. Several handles to the same object can exist at a given
/// time, and cloning them won't create a new protocol object, only
/// clone the handle. The lifetime of the protocol object is **not**
/// tied to the lifetime of these handles, but rather to sending or
/// receiving destroying messages.
///
/// These handles are notably used to send requests to the server. To do
/// you need to import the associated `RequestsTrait` trait from the module
/// of this interface.
pub struct Proxy<I: Interface> {
    _i: ::std::marker::PhantomData<&'static I>,
    pub(crate) inner: ProxyInner,
}

impl<I: Interface> Clone for Proxy<I> {
    fn clone(&self) -> Proxy<I> {
        Proxy {
            _i: ::std::marker::PhantomData,
            inner: self.inner.clone(),
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

    /// Send a request through this object
    ///
    /// **Warning:** This method is mostly intended to be used by code generated
    /// by `wayland-scanner`, and you should probably never need to use it directly,
    /// but rather use the appropriate `RequestsTrait` for your proxy.
    ///
    /// This is the generic method to send requests.
    ///
    /// If your request needs to create an object, use `send_constructor`.
    pub fn send(&self, msg: I::Request) {
        self.inner.send::<I>(msg)
    }

    /// Send a request creating an object through this object
    ///
    /// **Warning:** This method is mostly intended to be used by code generated
    /// by `wayland-scanner`, and you should probably never need to use it directly,
    /// but rather use the appropriate `RequestsTrait` for your proxy.
    ///
    /// This is the generic method to send requests that create objects
    ///
    /// The slot in the message corresponding with the newly created object must have
    /// been filled by a placeholder object (see `child_placeholder`).
    pub fn send_constructor<J, F>(
        &self,
        msg: I::Request,
        implementor: F,
        version: Option<u32>,
    ) -> Result<Proxy<J>, ()>
    where
        J: Interface,
        F: FnOnce(NewProxy<J>) -> Proxy<J>,
    {
        self.inner
            .send_constructor::<I, J>(msg, version)
            .map(NewProxy::wrap)
            .map(implementor)
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

    /// Access the arbitrary payload associated to this object
    ///
    /// You need to specify the expected type of this payload, and this
    /// function will return `None` if either the types don't match or
    /// you are attempting to access a non `Send + Sync` user data from the
    /// wrong thread.
    ///
    /// This value is associated to the Proxy when you implement it, and you
    /// cannot access it mutably afterwards. If you need interior mutability,
    /// you are responsible for using a `Mutex` or similar type to achieve it.
    pub fn user_data<UD: 'static>(&self) -> Option<&UD> {
        self.inner.get_user_data()
    }

    /// Check if the other proxy refers to the same underlying wayland object
    pub fn equals(&self, other: &Proxy<I>) -> bool {
        self.inner.equals(&other.inner)
    }

    /// Create a new child object
    ///
    /// **Warning:** This method is mostly intended to be used by code generated
    /// by `wayland-scanner`, and you should probably never need to use it directly,
    /// but rather use the appropriate `RequestsTrait` for your proxy.
    ///
    /// This creates a new wayland object, considered as a
    /// child of this object. It will notably inherit its interface
    /// version.
    ///
    /// The created object should immediately be implemented and sent
    /// in a request to the server, to keep the object list properly
    /// synchronized. Failure to do so will likely cause a protocol
    /// error.
    pub fn child<C: Interface>(&self) -> NewProxy<C> {
        NewProxy {
            _i: ::std::marker::PhantomData,
            inner: self.inner.child::<C>(),
        }
    }

    /// Creates a handle of this proxy with its actual type erased
    pub fn anonymize(&self) -> Proxy<AnonymousObject> {
        Proxy {
            _i: ::std::marker::PhantomData,
            inner: self.inner.clone(),
        }
    }

    /// Create a wrapper for this object for queue management
    ///
    /// As assigning a proxy to an event queue can be a racy operation
    /// in contexts involving multiple thread, this provides a facility
    /// to do this safely.
    ///
    /// The wrapper object created behaves like a regular `Proxy`, except that
    /// all objects created as the result of its requests will be assigned to
    /// the queue associated to the provided token, rather than the queue of
    /// their parent. This does not change the queue of the proxy itself.
    pub fn make_wrapper(&self, queue: &QueueToken) -> Result<Proxy<I>, ()> {
        let inner = self.inner.make_wrapper(&queue.inner)?;

        Ok(Proxy {
            _i: ::std::marker::PhantomData,
            inner,
        })
    }

    /// Create a placeholder object, to be used with `send_constructor`
    ///
    /// **Warning:** This method is mostly intended to be used by code generated
    /// by `wayland-scanner`, and you should probably never need to use it directly,
    /// but rather use the appropriate `RequestsTrait` for your proxy.
    pub fn child_placeholder<J: Interface>(&self) -> Proxy<J> {
        Proxy::wrap(self.inner.child_placeholder())
    }
}

#[cfg(feature = "native_lib")]
impl<I: Interface> Proxy<I> {
    /// Check whether this proxy is managed by the library or not
    ///
    /// See `from_c_ptr` for details.
    pub fn is_external(&self) -> bool {
        self.inner.is_external()
    }

    /// Get a raw pointer to the underlying wayland object
    ///
    /// Retrieve a pointer to the object from the `libwayland-client.so` library.
    /// You will mostly need it to interface with C libraries needing access
    /// to wayland objects (to initialize an opengl context for example).
    pub fn c_ptr(&self) -> *mut wl_proxy {
        self.inner.c_ptr()
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
    pub unsafe fn from_c_ptr(ptr: *mut wl_proxy) -> Proxy<I> {
        Proxy {
            _i: ::std::marker::PhantomData,
            inner: ProxyInner::from_c_ptr::<I>(ptr),
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

/// A newly-created proxy that needs implementation
///
/// Whenever a new wayland object is created, you will
/// receive it as a `NewProxy`. You then have to provide an
/// implementation for it, in order to process the incoming
/// events it may receive. Once this done you will be able
/// to use it as a regular `Proxy`.
///
/// Implementations are structs implementing the appropriate
/// variant of the `Implementation` trait. They can also be
/// closures.
pub struct NewProxy<I: Interface> {
    _i: ::std::marker::PhantomData<*const I>,
    pub(crate) inner: NewProxyInner,
}

impl<I: Interface + 'static> NewProxy<I> {
    #[allow(dead_code)]
    pub(crate) fn wrap(inner: NewProxyInner) -> NewProxy<I> {
        NewProxy {
            _i: ::std::marker::PhantomData,
            inner,
        }
    }

    /// Implement this proxy using given function and implementation data.
    pub fn implement<F, UD>(self, implementation: F, user_data: UD) -> Proxy<I>
    where
        F: FnMut(I::Event, Proxy<I>) + Send + 'static,
        UD: Send + Sync + 'static,
        I::Event: MessageGroup<Map = ProxyMap>,
    {
        let inner = unsafe {
            self.inner
                .implement::<I, _>(implementation, UserData::new_threadsafe(user_data))
        };
        Proxy {
            _i: ::std::marker::PhantomData,
            inner: inner,
        }
    }

    /// Implement this proxy using given function and implementation data.
    ///
    /// This method allows the implementation to not be `Send`, but requires for
    /// safety that you provide a token to the event queue this proxy will be implemented
    /// on. This method will then ensure that this proxy is registered on this event queue,
    /// so that it cannot be dispatched from an other thread.
    ///
    /// **Unsafety:**
    ///
    /// This call can be racy if the proxy is not already registered on this event queue and its
    /// old queue is being dispatched from an other thread.
    ///
    /// To ensure safety, see `Proxy::make_wrapper`.
    pub unsafe fn implement_nonsend<Impl, UD>(
        self,
        implementation: Impl,
        user_data: UD,
        queue: &QueueToken,
    ) -> Proxy<I>
    where
        Impl: FnMut(I::Event, Proxy<I>) + 'static,
        UD: 'static,
        I::Event: MessageGroup<Map = ProxyMap>,
    {
        #[cfg(feature = "native_lib")]
        {
            queue.inner.assign_proxy(self.inner.c_ptr());
        }
        #[cfg(not(feature = "native_lib"))]
        {
            self.inner.assign_queue(&queue.inner);
        }
        let inner = self
            .inner
            .implement::<I, _>(implementation, UserData::new(user_data));
        Proxy {
            _i: ::std::marker::PhantomData,
            inner: inner,
        }
    }
}

#[cfg(feature = "native_lib")]
impl<I: Interface + 'static> NewProxy<I> {
    /// Get a raw pointer to the underlying wayland object
    ///
    /// Retrieve a pointer to the object from the `libwayland-client.so` library.
    /// You will mostly need it to interface with C libraries needing access
    /// to wayland objects (to initialize an opengl context for example).
    ///
    /// Use this if you need to pass an unimplemented object to the C library
    /// you are interfacing with.
    pub fn c_ptr(&self) -> *mut wl_proxy {
        self.inner.c_ptr()
    }

    /// Create a `NewProxy` instance from a C pointer.
    ///
    /// By doing so, you assert that this wayland object was newly created and
    /// can be safely implemented. As implementing it will overwrite any previously
    /// associated data or implementation, this can cause weird errors akin to
    /// memory corruption if it was not the case.
    pub unsafe fn from_c_ptr(ptr: *mut wl_proxy) -> Self {
        NewProxy {
            _i: ::std::marker::PhantomData,
            inner: NewProxyInner::from_c_ptr(ptr),
        }
    }
}
