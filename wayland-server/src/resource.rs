use wayland_commons::utils::UserData;
use wayland_commons::{Interface, MessageGroup};

use {Client, DisplayToken};

#[cfg(feature = "native_lib")]
use wayland_sys::server::*;

use imp::{NewResourceInner, ResourceInner};

/// An handle to a wayland resource
///
/// This represents a wayland object instantiated in a client
/// session. Several handles to the same object can exist at a given
/// time, and cloning them won't create a new protocol object, only
/// clone the handle. The lifetime of the protocol object is **not**
/// tied to the lifetime of these handles, but rather to sending or
/// receiving destroying messages.
///
/// These handles are notably used to send events to the associated client,
/// via the `send` method.
pub struct Resource<I: Interface> {
    _i: ::std::marker::PhantomData<&'static I>,
    inner: ResourceInner,
}

impl<I: Interface> PartialEq for Resource<I> {
    fn eq(&self, other: &Resource<I>) -> bool {
        self.equals(other)
    }
}

impl<I: Interface> Eq for Resource<I> {}

impl<I: Interface> Resource<I> {
    #[allow(dead_code)]
    pub(crate) fn wrap(inner: ResourceInner) -> Resource<I> {
        Resource {
            _i: ::std::marker::PhantomData,
            inner,
        }
    }

    /// Send an event through this object
    ///
    /// The event will be send to the client associated to this
    /// object.
    pub fn send(&self, msg: I::Event) {
        self.inner.send::<I>(msg)
    }

    /// Check if the object associated with this resource is still alive
    ///
    /// Will return `false` if either:
    ///
    /// - The object has been destroyed
    /// - The object is not managed by this library (see the `from_c_ptr` method)
    pub fn is_alive(&self) -> bool {
        self.inner.is_alive()
    }

    /// Retrieve the interface version of this wayland object instance
    ///
    /// Returns 0 on dead objects
    pub fn version(&self) -> u32 {
        self.inner.version()
    }

    /// Check if the other resource refers to the same underlying wayland object
    pub fn equals(&self, other: &Resource<I>) -> bool {
        self.inner.equals(&other.inner)
    }

    /// Check if this resource and the other belong to the same client
    ///
    /// Always return false if either of them is dead
    pub fn same_client_as<II: Interface>(&self, other: &Resource<II>) -> bool {
        self.inner.same_client_as(&other.inner)
    }

    /// Posts a protocol error to this resource
    ///
    /// The error code can be obtained from the various `Error` enums of the protocols.
    ///
    /// An error is fatal to the client that caused it.
    pub fn post_error(&self, error_code: u32, msg: String) {
        self.inner.post_error(error_code, msg)
    }

    /// Access the arbitrary payload associated to this object
    ///
    /// You need to specify the expected type of this payload, and this
    /// function will return `None` if either the types don't match or
    /// you are attempting to access a non `Send + Sync` user data from the
    /// wrong thread.
    ///
    /// This value is associated to the Resource when you implement it, and you
    /// cannot access it mutably afterwards. If you need interior mutability,
    /// you are responsible for using a `Mutex` or similar type to achieve it.
    pub fn user_data<UD: 'static>(&self) -> Option<&UD> {
        self.inner.get_user_data()
    }

    /// Retrieve an handle to the client associated with this resource
    ///
    /// Returns `None` if the resource is no longer alive.
    pub fn client(&self) -> Option<Client> {
        self.inner.client().map(Client::make)
    }

    /// Retrieve the object id of this wayland object
    pub fn id(&self) -> u32 {
        self.inner.id()
    }
}

#[cfg(feature = "native_lib")]
impl<I: Interface> Resource<I> {
    /// Check whether this resource is managed by the library or not
    ///
    /// See `from_c_ptr` for details.
    pub fn is_external(&self) -> bool {
        self.inner.is_external()
    }

    /// Get a raw pointer to the underlying wayland object
    ///
    /// Retrieve a pointer to the object from the `libwayland-server.so` library.
    /// You will mostly need it to interface with C libraries needing access
    /// to wayland objects (to initialize an opengl context for example).
    pub fn c_ptr(&self) -> *mut wl_resource {
        self.inner.c_ptr()
    }

    /// Create a `Resource` instance from a C pointer
    ///
    /// Create a `Resource` from a raw pointer to a wayland object from the
    /// C library.
    ///
    /// If the pointer was previously obtained by the `c_ptr()` method, this
    /// constructs a new resource handle for the same object just like the
    /// `clone()` method would have.
    ///
    /// If the object was created by some other C library you are interfacing
    /// with, it will be created in an "unmanaged" state: wayland-server will
    /// treat it as foreign, and as such most of the safeties will be absent.
    /// Notably the lifetime of the object can't be tracked, so the `alive()`
    /// method will always return `false` and you are responsible of not using
    /// an object past its destruction (as this would cause a protocol error).
    /// You will also be unable to associate any user data pointer to this object.
    ///
    /// In order to handle protocol races, invoking it with a NULL pointer will
    /// create an already-dead object.
    pub unsafe fn from_c_ptr(ptr: *mut wl_resource) -> Self {
        Resource {
            _i: ::std::marker::PhantomData,
            inner: ResourceInner::from_c_ptr::<I>(ptr),
        }
    }
}

/// A newly-created resource that needs implementation
///
/// Whenever a new wayland object is created, you will
/// receive it as a `NewResource`. You then have to provide an
/// implementation for it, in order to process the incoming
/// events it may receive. Once this done you will be able
/// to use it as a regular `Resource`.
///
/// Implementations are structs implementing the appropriate
/// variant of the `Implementation` trait. They can also be
/// closures.
pub struct NewResource<I: Interface> {
    _i: ::std::marker::PhantomData<*const I>,
    inner: NewResourceInner,
}

impl<I: Interface + 'static> NewResource<I> {
    pub(crate) fn wrap(inner: NewResourceInner) -> NewResource<I> {
        NewResource {
            _i: ::std::marker::PhantomData,
            inner,
        }
    }

    /// Implement this resource using given function, destructor, and user data.
    pub fn implement<F, Dest, UD>(
        self,
        implementation: F,
        destructor: Option<Dest>,
        user_data: UD,
    ) -> Resource<I>
    where
        F: FnMut(I::Request, Resource<I>) + Send + 'static,
        Dest: FnMut(Resource<I>) + Send + 'static,
        UD: Send + Sync + 'static,
        I::Request: MessageGroup<Map = ::imp::ResourceMap>,
    {
        let inner = unsafe {
            self.inner.implement::<I, F, Dest>(
                implementation,
                destructor,
                UserData::new_threadsafe(user_data),
            )
        };
        Resource {
            _i: ::std::marker::PhantomData,
            inner,
        }
    }

    /// Implement this resource using given function and implementation data.
    ///
    /// This method allows the implementation data to not be `Send`, but requires for
    /// safety that you provide a token to the event loop owning the proxy. As the token
    /// is not `Send`, this ensures you are implementing the resource from the same thread
    /// as the event loop runs on.
    ///
    /// ** Panics **
    ///
    /// This function will panic if you create several wayland displays and do not
    /// provide a token to the right one.
    pub fn implement_nonsend<F, Dest, UD>(
        self,
        implementation: F,
        destructor: Option<Dest>,
        user_data: UD,
        token: &DisplayToken,
    ) -> Resource<I>
    where
        F: FnMut(I::Request, Resource<I>) + 'static,
        Dest: FnMut(Resource<I>) + 'static,
        UD: 'static,
        I::Request: MessageGroup<Map = ::imp::ResourceMap>,
    {
        let display = token
            .upgrade()
            .expect("Attempted to implement a resource on a dead Display.");
        if !self.inner.on_display(&*display.borrow()) {
            panic!("Attempted to implement a resource with the wrong DisplayToken.")
        }
        let inner = unsafe {
            self.inner
                .implement::<I, F, Dest>(implementation, destructor, UserData::new(user_data))
        };
        Resource {
            _i: ::std::marker::PhantomData,
            inner,
        }
    }
}

#[cfg(feature = "native_lib")]
impl<I: Interface> NewResource<I> {
    /// Get a raw pointer to the underlying wayland object
    ///
    /// Retrieve a pointer to the object from the `libwayland-server.so` library.
    /// You will mostly need it to interface with C libraries needing access
    /// to wayland objects.
    ///
    /// Use this if you need to pass an unimplemented object to the C library
    /// you are interfacing with.
    pub fn c_ptr(&self) -> *mut wl_resource {
        self.inner.c_ptr()
    }

    /// Create a `NewResource` instance from a C pointer.
    ///
    /// By doing so, you assert that this wayland object was newly created and
    /// can be safely implemented. As implementing it will overwrite any previously
    /// associated data or implementation, this can cause weird errors akin to
    /// memory corruption if it was not the case.
    pub unsafe fn from_c_ptr(ptr: *mut wl_resource) -> Self {
        NewResource {
            _i: ::std::marker::PhantomData,
            inner: NewResourceInner::from_c_ptr(ptr),
        }
    }
}

impl<I: Interface> Clone for Resource<I> {
    fn clone(&self) -> Resource<I> {
        Resource {
            _i: ::std::marker::PhantomData,
            inner: self.inner.clone(),
        }
    }
}

/// Provides a callback function to handle requests of the implementing interface via `T`.
///
/// This trait is meant to be implemented automatically by code generated with `wayland-scanner`.
pub trait HandledBy<T>: Interface + Sized {
    /// Handles an event.
    fn handle(handler: &mut T, request: Self::Request, resource: Resource<Self>);
}
