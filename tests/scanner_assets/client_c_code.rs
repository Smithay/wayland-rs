//
// This file was auto-generated, do not edit directly.
//

/*
This is an example copyright.
    It contains several lines.
    AS WELL AS ALL CAPS TEXT.
*/

pub mod wl_foo {
    //! Interface for fooing
    //!
    //! This is the dedicated interface for doing foos over any
    //! kind of other foos.

    use super::{Proxy, NewProxy, AnonymousObject, Interface, MessageGroup, MessageDesc, ArgumentType};
    use super::sys::common::{wl_argument, wl_interface, wl_array};
    use super::sys::client::*;

    /// Possible cake kinds
    ///
    /// List of the possible kind of cake supported by the protocol.

    #[repr(u32)]
    #[derive(Copy,Clone,Debug,PartialEq)]
    pub enum CakeKind {
        /// mild cake without much flavor
        Basic = 0,
        /// spicy cake to burn your tongue
        Spicy = 1,
        /// fruity cake to get vitamins
        Fruity = 2,
    }

    impl CakeKind {
        pub fn from_raw(n: u32) -> Option<CakeKind> {
            match n {
                0 => Some(CakeKind::Basic),
                1 => Some(CakeKind::Spicy),
                2 => Some(CakeKind::Fruity),

                _ => Option::None
            }
        }
        pub fn to_raw(&self) -> u32 {
            *self as u32
        }
    }

    bitflags! {
        /// possible delivery modes
        ///
        pub struct DeliveryKind: u32 {
            /// pick your cake up yourself
            const PickUp = 1;
            /// flying drone delivery
            const Drone = 2;
            /// because we fear nothing
            const Catapult = 4;
        }
    }

    impl DeliveryKind {
        pub fn from_raw(n: u32) -> Option<DeliveryKind> {
            Some(DeliveryKind::from_bits_truncate(n))

        }
        pub fn to_raw(&self) -> u32 {
            self.bits()
        }
    }

    pub enum Request {
        /// do some foo
        ///
        /// This will do some foo with its args.
        FooIt {number: i32, unumber: u32, text: String, float: f64, file: ::std::os::unix::io::RawFd, },
        /// create a bar
        ///
        /// Create a bar which will do its bar job.
        CreateBar {id: Proxy<super::wl_bar::WlBar>, },
    }

    impl super::MessageGroup for Request {
        const MESSAGES: &'static [super::MessageDesc] = &[
            super::MessageDesc {
                name: "foo_it",
                since: 1,
                signature: &[
                    super::ArgumentType::Int,
                    super::ArgumentType::Uint,
                    super::ArgumentType::Str,
                    super::ArgumentType::Fixed,
                    super::ArgumentType::Fd,
                ]
            },
            super::MessageDesc {
                name: "create_bar",
                since: 1,
                signature: &[
                    super::ArgumentType::NewId,
                ]
            },
        ];

        fn is_destructor(&self) -> bool {
            match *self {
                _ => false
            }
        }

        unsafe fn from_raw_c(obj: *mut ::std::os::raw::c_void, opcode: u32, args: *const wl_argument) -> Result<Request,()> {
            panic!("Request::from_raw_c can not be used Client-side.")
        }

        fn as_raw_c_in<F, T>(self, f: F) -> T where F: FnOnce(u32, &mut [wl_argument]) -> T {
            match self {
                Request::FooIt { number, unumber, text, float, file, } => {
                    let mut _args_array: [wl_argument; 5] = unsafe { ::std::mem::zeroed() };
                    _args_array[0].i = number;
                    _args_array[1].u = unumber;
                    let _arg_2 = ::std::ffi::CString::new(text).unwrap();
                    _args_array[2].s = _arg_2.as_ptr();
                    _args_array[3].f = (float * 256.) as i32;
                    _args_array[4].h = file;
                    f(0, &mut _args_array)
                },
                Request::CreateBar { id, } => {
                    let mut _args_array: [wl_argument; 1] = unsafe { ::std::mem::zeroed() };
                    _args_array[0].o = id.c_ptr() as *mut _;
                    f(1, &mut _args_array)
                },
            }
        }
    }

    pub enum Event {
        /// a cake is possible
        ///
        /// The server advertizes that a kind of cake is available
        ///
        /// Only available since version 2 of the interface
        Cake {kind: CakeKind, amount: u32, },
    }

    impl super::MessageGroup for Event {
        const MESSAGES: &'static [super::MessageDesc] = &[
            super::MessageDesc {
                name: "cake",
                since: 2,
                signature: &[
                    super::ArgumentType::Uint,
                    super::ArgumentType::Uint,
                ]
            },
        ];

        fn is_destructor(&self) -> bool {
            match *self {
                _ => false
            }
        }

        unsafe fn from_raw_c(obj: *mut ::std::os::raw::c_void, opcode: u32, args: *const wl_argument) -> Result<Event,()> {
            match opcode {
                0 => {
                    let _args = ::std::slice::from_raw_parts(args, 2);
                    Ok(Event::Cake {
                        kind: CakeKind::from_raw(_args[0].u).ok_or(())?,
                        amount: _args[1].u,
                }) },
                _ => return Err(())
            }
        }

        fn as_raw_c_in<F, T>(self, f: F) -> T where F: FnOnce(u32, &mut [wl_argument]) -> T {
            panic!("Event::as_raw_c_in can not be used Client-side.")
        }
    }


    pub struct WlFoo;

    impl Interface for WlFoo {
        type Request = Request;
        type Event = Event;
        const NAME: &'static str = "wl_foo";
        const VERSION: u32 = 3;
        fn c_interface() -> *const wl_interface {
            unsafe { &super::super::c_interfaces::wl_foo_interface }
        }

    }

    pub trait RequestsTrait {
        /// do some foo
        ///
        /// This will do some foo with its args.
        fn foo_it(&self, number: i32, unumber: u32, text: String, float: f64, file: ::std::os::unix::io::RawFd) ->();
        /// create a bar
        ///
        /// Create a bar which will do its bar job.
        fn create_bar(&self) ->Result<NewProxy<super::wl_bar::WlBar>, ()>;
    }

    impl RequestsTrait for Proxy<WlFoo> {
        fn foo_it(&self, number: i32, unumber: u32, text: String, float: f64, file: ::std::os::unix::io::RawFd) ->() {
            if !self.is_external() && !self.is_alive() {
                return;
            }
            let msg = Request::FooIt {
                number: number,
                unumber: unumber,
                text: text,
                float: float,
                file: file,
            };
            self.send(msg);
        }

        fn create_bar(&self) ->Result<NewProxy<super::wl_bar::WlBar>, ()> {
            if !self.is_external() && !self.is_alive() {
                return Err(());
            }
            let _arg_id_newproxy = self.child::<super::wl_bar::WlBar>();
            let msg = Request::CreateBar {
                id: unsafe { Proxy::<super::wl_bar::WlBar>::from_c_ptr(_arg_id_newproxy.c_ptr()) },
            };
            self.send(msg);
            Ok(_arg_id_newproxy)
        }
    }
}

pub mod wl_bar {
    //! Interface for bars
    //!
    //! This interface allows you to bar your foos.

    use super::{Proxy, NewProxy, AnonymousObject, Interface, MessageGroup, MessageDesc, ArgumentType};
    use super::sys::common::{wl_argument, wl_interface, wl_array};
    use super::sys::client::*;

    pub enum Request {
        /// ask for a bar delivery
        ///
        /// Proceed to a bar delivery of given foo.
        ///
        /// Only available since version 2 of the interface
        BarDelivery {kind: super::wl_foo::DeliveryKind, target: Proxy<super::wl_foo::WlFoo>, metadata: Vec<u8>, },
        /// release this bar
        ///
        /// Notify the compositor that you have finished using this bar.
        ///
        /// This is a destructor, once sent this object cannot be used any longer.
        Release,
    }

    impl super::MessageGroup for Request {
        const MESSAGES: &'static [super::MessageDesc] = &[
            super::MessageDesc {
                name: "bar_delivery",
                since: 2,
                signature: &[
                    super::ArgumentType::Uint,
                    super::ArgumentType::Object,
                    super::ArgumentType::Array,
                ]
            },
            super::MessageDesc {
                name: "release",
                since: 1,
                signature: &[
                ]
            },
        ];

        fn is_destructor(&self) -> bool {
            match *self {
                Request::Release => true,
                _ => false
            }
        }

        unsafe fn from_raw_c(obj: *mut ::std::os::raw::c_void, opcode: u32, args: *const wl_argument) -> Result<Request,()> {
            panic!("Request::from_raw_c can not be used Client-side.")
        }

        fn as_raw_c_in<F, T>(self, f: F) -> T where F: FnOnce(u32, &mut [wl_argument]) -> T {
            match self {
                Request::BarDelivery { kind, target, metadata, } => {
                    let mut _args_array: [wl_argument; 3] = unsafe { ::std::mem::zeroed() };
                    _args_array[0].u = kind.to_raw();
                    _args_array[1].o = target.c_ptr() as *mut _;
                    let _arg_2 = wl_array { size: metadata.len(), alloc: metadata.capacity(), data: metadata.as_ptr() as *mut _ };
                    _args_array[2].a = &_arg_2;
                    f(0, &mut _args_array)
                },
                Request::Release => {
                    let mut _args_array: [wl_argument; 0] = unsafe { ::std::mem::zeroed() };
                    f(1, &mut _args_array)
                },
            }
        }
    }

    pub enum Event {
    }

    impl super::MessageGroup for Event {
        const MESSAGES: &'static [super::MessageDesc] = &[
        ];

        fn is_destructor(&self) -> bool {
            match *self {
            }
        }

        unsafe fn from_raw_c(obj: *mut ::std::os::raw::c_void, opcode: u32, args: *const wl_argument) -> Result<Event,()> {
            match opcode {
                _ => return Err(())
            }
        }

        fn as_raw_c_in<F, T>(self, f: F) -> T where F: FnOnce(u32, &mut [wl_argument]) -> T {
            panic!("Event::as_raw_c_in can not be used Client-side.")
        }
    }


    pub struct WlBar;

    impl Interface for WlBar {
        type Request = Request;
        type Event = Event;
        const NAME: &'static str = "wl_bar";
        const VERSION: u32 = 1;
        fn c_interface() -> *const wl_interface {
            unsafe { &super::super::c_interfaces::wl_bar_interface }
        }

    }

    pub trait RequestsTrait {
        /// ask for a bar delivery
        ///
        /// Proceed to a bar delivery of given foo.
        ///
        /// Only available since version 2 of the interface
        fn bar_delivery(&self, kind: super::wl_foo::DeliveryKind, target: &Proxy<super::wl_foo::WlFoo>, metadata: Vec<u8>) ->();
        /// release this bar
        ///
        /// Notify the compositor that you have finished using this bar.
        ///
        /// This is a destructor, you cannot send requests to this object any longer once this method is called.
        fn release(&self) ->();
    }

    impl RequestsTrait for Proxy<WlBar> {
        fn bar_delivery(&self, kind: super::wl_foo::DeliveryKind, target: &Proxy<super::wl_foo::WlFoo>, metadata: Vec<u8>) ->() {
            if !self.is_external() && !self.is_alive() {
                return;
            }

            let msg = Request::BarDelivery {
                kind: kind,
                target: target.clone(),
                metadata: metadata,
            };
            self.send(msg);
        }

        fn release(&self) ->() {
            if !self.is_external() && !self.is_alive() {
                return;
            }
            let msg = Request::Release;
            self.send(msg);
        }
    }
}

pub mod wl_display {
    //! core global object
    //!
    //! This global is special and should only generate code client-side, not server-side.

    use super::{Proxy, NewProxy, AnonymousObject, Interface, MessageGroup, MessageDesc, ArgumentType};
    use super::sys::common::{wl_argument, wl_interface, wl_array};
    use super::sys::client::*;

    pub enum Request {
    }

    impl super::MessageGroup for Request {
        const MESSAGES: &'static [super::MessageDesc] = &[
        ];

        fn is_destructor(&self) -> bool {
            match *self {
            }
        }

        unsafe fn from_raw_c(obj: *mut ::std::os::raw::c_void, opcode: u32, args: *const wl_argument) -> Result<Request,()> {
            panic!("Request::from_raw_c can not be used Client-side.")
        }

        fn as_raw_c_in<F, T>(self, f: F) -> T where F: FnOnce(u32, &mut [wl_argument]) -> T {
            match self {
            }
        }
    }

    pub enum Event {
    }

    impl super::MessageGroup for Event {
        const MESSAGES: &'static [super::MessageDesc] = &[
        ];

        fn is_destructor(&self) -> bool {
            match *self {
            }
        }

        unsafe fn from_raw_c(obj: *mut ::std::os::raw::c_void, opcode: u32, args: *const wl_argument) -> Result<Event,()> {
            match opcode {
                _ => return Err(())
            }
        }

        fn as_raw_c_in<F, T>(self, f: F) -> T where F: FnOnce(u32, &mut [wl_argument]) -> T {
            panic!("Event::as_raw_c_in can not be used Client-side.")
        }
    }


    pub struct WlDisplay;

    impl Interface for WlDisplay {
        type Request = Request;
        type Event = Event;
        const NAME: &'static str = "wl_display";
        const VERSION: u32 = 1;
        fn c_interface() -> *const wl_interface {
            unsafe { &super::super::c_interfaces::wl_display_interface }
        }

    }

    pub trait RequestsTrait {
    }

    impl RequestsTrait for Proxy<WlDisplay> {
    }
}

pub mod wl_registry {
    //! global registry object
    //!
    //! This global is special and should only generate code client-side, not server-side.

    use super::{Proxy, NewProxy, AnonymousObject, Interface, MessageGroup, MessageDesc, ArgumentType};
    use super::sys::common::{wl_argument, wl_interface, wl_array};
    use super::sys::client::*;

    pub enum Request {
        /// bind an object to the display
        ///
        /// This request is a special code-path, as its new-id argument as no target type.
        Bind {name: u32, id: (String, u32, Proxy<AnonymousObject>), },
    }

    impl super::MessageGroup for Request {
        const MESSAGES: &'static [super::MessageDesc] = &[
            super::MessageDesc {
                name: "bind",
                since: 1,
                signature: &[
                    super::ArgumentType::Uint,
                    super::ArgumentType::NewId,
                ]
            },
        ];

        fn is_destructor(&self) -> bool {
            match *self {
                _ => false
            }
        }

        unsafe fn from_raw_c(obj: *mut ::std::os::raw::c_void, opcode: u32, args: *const wl_argument) -> Result<Request,()> {
            panic!("Request::from_raw_c can not be used Client-side.")
        }

        fn as_raw_c_in<F, T>(self, f: F) -> T where F: FnOnce(u32, &mut [wl_argument]) -> T {
            match self {
                Request::Bind { name, id, } => {
                    let mut _args_array: [wl_argument; 4] = unsafe { ::std::mem::zeroed() };
                    _args_array[0].u = name;
                    let _arg_1_s = ::std::ffi::CString::new(id.0).unwrap();
                    _args_array[1].s = _arg_1_s.as_ptr();
                    _args_array[2].u = id.1;
                    _args_array[3].o = ::std::ptr::null_mut();
                    f(0, &mut _args_array)
                },
            }
        }
    }

    pub enum Event {
    }

    impl super::MessageGroup for Event {
        const MESSAGES: &'static [super::MessageDesc] = &[
        ];

        fn is_destructor(&self) -> bool {
            match *self {
            }
        }

        unsafe fn from_raw_c(obj: *mut ::std::os::raw::c_void, opcode: u32, args: *const wl_argument) -> Result<Event,()> {
            match opcode {
                _ => return Err(())
            }
        }

        fn as_raw_c_in<F, T>(self, f: F) -> T where F: FnOnce(u32, &mut [wl_argument]) -> T {
            panic!("Event::as_raw_c_in can not be used Client-side.")
        }
    }


    pub struct WlRegistry;

    impl Interface for WlRegistry {
        type Request = Request;
        type Event = Event;
        const NAME: &'static str = "wl_registry";
        const VERSION: u32 = 1;
        fn c_interface() -> *const wl_interface {
            unsafe { &super::super::c_interfaces::wl_registry_interface }
        }

    }

    pub trait RequestsTrait {
        /// bind an object to the display
        ///
        /// This request is a special code-path, as its new-id argument as no target type.
        fn bind<T: Interface>(&self, version: u32, name: u32) ->Result<NewProxy<T>, ()>;
    }

    impl RequestsTrait for Proxy<WlRegistry> {
        fn bind<T: Interface>(&self, version: u32, name: u32) ->Result<NewProxy<T>, ()> {
            if !self.is_external() && !self.is_alive() {
                return Err(());
            }
            let msg = Request::Bind {
                name: name,
                id: (T::NAME.into(), version, unsafe { Proxy::<AnonymousObject>::new_null() }),
            };

            unsafe {
                let ret = msg.as_raw_c_in(|opcode, args| {
                    ffi_dispatch!(
                        WAYLAND_CLIENT_HANDLE,
                        wl_proxy_marshal_array_constructor_versioned,
                        self.c_ptr(),
                        opcode,
                        args.as_mut_ptr(),
                        T::c_interface(),
                        version
                    )
                });
                Ok(NewProxy::<T>::from_c_ptr(ret))
            }
        }
    }
}

pub mod wl_callback {
    //! callback object
    //!
    //! This object has a special behavior regarding its destructor.

    use super::{Proxy, NewProxy, AnonymousObject, Interface, MessageGroup, MessageDesc, ArgumentType};
    use super::sys::common::{wl_argument, wl_interface, wl_array};
    use super::sys::client::*;

    pub enum Request {
    }

    impl super::MessageGroup for Request {
        const MESSAGES: &'static [super::MessageDesc] = &[
        ];
        fn is_destructor(&self) -> bool {
            match *self {
            }
        }

        unsafe fn from_raw_c(obj: *mut ::std::os::raw::c_void, opcode: u32, args: *const wl_argument) -> Result<Request,()> {
            panic!("Request::from_raw_c can not be used Client-side.")
        }

        fn as_raw_c_in<F, T>(self, f: F) -> T where F: FnOnce(u32, &mut [wl_argument]) -> T {
            match self {
            }
        }
    }

    pub enum Event {
        /// done event
        ///
        /// This event is actually a destructor, but the protocol XML has no wait of specifying it.
        /// As such, the scanner should consider wl_callback.done as a special case.
        ///
        /// This is a destructor, once received this object cannot be used any longer.
        Done {callback_data: u32, },
    }

    impl super::MessageGroup for Event {
        const MESSAGES: &'static [super::MessageDesc] = &[
            super::MessageDesc {
                name: "done",
                since: 1,
                signature: &[
                    super::ArgumentType::Uint,
                ]
            },
        ];

        fn is_destructor(&self) -> bool {
            match *self {
                Event::Done { .. } => true,
            }
        }

        unsafe fn from_raw_c(obj: *mut ::std::os::raw::c_void, opcode: u32, args: *const wl_argument) -> Result<Event,()> {
            match opcode {
                0 => {
                    let _args = ::std::slice::from_raw_parts(args, 1);
                    Ok(Event::Done {
                        callback_data: _args[0].u,
                }) },
                _ => return Err(())
            }
        }

        fn as_raw_c_in<F, T>(self, f: F) -> T where F: FnOnce(u32, &mut [wl_argument]) -> T {
            panic!("Event::as_raw_c_in can not be used Client-side.")
        }
    }


    pub struct WlCallback;

    impl Interface for WlCallback {
        type Request = Request;
        type Event = Event;
        const NAME: &'static str = "wl_callback";
        const VERSION: u32 = 1;
        fn c_interface() -> *const wl_interface {
            unsafe { &super::super::c_interfaces::wl_callback_interface }
        }

    }

    pub trait RequestsTrait {
    }

    impl RequestsTrait for Proxy<WlCallback> {
    }
}

