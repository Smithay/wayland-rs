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

    use super::{Resource, NewResource, AnonymousObject, Interface, MessageGroup, MessageDesc, ArgumentType};
    use super::sys::common::{wl_argument, wl_interface, wl_array};
    use super::sys::server::*;

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
        CreateBar {id: NewResource<super::wl_bar::WlBar>, },
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
            match opcode {
                0 => {
                    let _args = ::std::slice::from_raw_parts(args, 5);
                    Ok(Request::FooIt {
                        number: _args[0].i,
                        unumber: _args[1].u,
                        text: ::std::ffi::CStr::from_ptr(_args[2].s).to_string_lossy().into_owned(),
                        float: (_args[3].f as f64)/256.,
                        file: _args[4].h,
                }) },
                1 => {
                    let _args = ::std::slice::from_raw_parts(args, 1);
                    Ok(Request::CreateBar {
                        id: { let client = ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_resource_get_client, obj as *mut _); let version = ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_resource_get_version, obj as *mut _); let new_ptr = ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_resource_create, client, super::wl_bar::WlBar::c_interface(), version, _args[0].n);NewResource::<super::wl_bar::WlBar>::from_c_ptr(new_ptr) },
                }) },
                _ => return Err(())
            }
        }

        fn as_raw_c_in<F, T>(self, f: F) -> T where F: FnOnce(u32, &mut [wl_argument]) -> T {
            panic!("Request::as_raw_c_in can not be used Server-side.")
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
            panic!("Event::from_raw_c can not be used Server-side.")
        }

        fn as_raw_c_in<F, T>(self, f: F) -> T where F: FnOnce(u32, &mut [wl_argument]) -> T {
            match self {
                Event::Cake { kind, amount, } => {
                    let mut _args_array: [wl_argument; 2] = unsafe { ::std::mem::zeroed() };
                    _args_array[0].u = kind.to_raw();
                    _args_array[1].u = amount;
                    f(0, &mut _args_array)
                },
            }
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
}

pub mod wl_bar {
    //! Interface for bars
    //!
    //! This interface allows you to bar your foos.

    use super::{Resource, NewResource, AnonymousObject, Interface, MessageGroup, MessageDesc, ArgumentType};
    use super::sys::common::{wl_argument, wl_interface, wl_array};
    use super::sys::server::*;

    pub enum Request {
        /// ask for a bar delivery
        ///
        /// Proceed to a bar delivery of given foo.
        ///
        /// Only available since version 2 of the interface
        BarDelivery {kind: super::wl_foo::DeliveryKind, target: Resource<super::wl_foo::WlFoo>, metadata: Vec<u8>, },
        /// release this bar
        ///
        /// Notify the compositor that you have finished using this bar.
        ///
        /// This is a destructor, once received this object cannot be used any longer.
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
            match opcode {
                0 => {
                    let _args = ::std::slice::from_raw_parts(args, 3);
                    Ok(Request::BarDelivery {
                        kind: super::wl_foo::DeliveryKind::from_raw(_args[0].u).ok_or(())?,
                        target: Resource::<super::wl_foo::WlFoo>::from_c_ptr(_args[1].o as *mut _),
                        metadata: { let array = &*_args[2].a; ::std::slice::from_raw_parts(array.data as *const u8, array.size).to_owned() },
                }) },
                1 => {
                    Ok(Request::Release) },
                _ => return Err(())
            }
        }

        fn as_raw_c_in<F, T>(self, f: F) -> T where F: FnOnce(u32, &mut [wl_argument]) -> T {
            panic!("Request::as_raw_c_in can not be used Server-side.")
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
            panic!("Event::from_raw_c can not be used Server-side.")
        }

        fn as_raw_c_in<F, T>(self, f: F) -> T where F: FnOnce(u32, &mut [wl_argument]) -> T {
            match self {
            }
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
}

pub mod wl_callback {
    //! callback object
    //!
    //! This object has a special behavior regarding its destructor.

    use super::{Resource, NewResource, AnonymousObject, Interface, MessageGroup, MessageDesc, ArgumentType};
    use super::sys::common::{wl_argument, wl_interface, wl_array};
    use super::sys::server::*;

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
            match opcode {
                _ => return Err(())
            }
        }

        fn as_raw_c_in<F, T>(self, f: F) -> T where F: FnOnce(u32, &mut [wl_argument]) -> T {
            panic!("Request::as_raw_c_in can not be used Server-side.")
        }
    }

    pub enum Event {
        /// done event
        ///
        /// This event is actually a destructor, but the protocol XML has no wait of specifying it.
        /// As such, the scanner should consider wl_callback.done as a special case.
        ///
        /// This is a destructor, once sent this object cannot be used any longer.
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
            panic!("Event::from_raw_c can not be used Server-side.")
        }

        fn as_raw_c_in<F, T>(self, f: F) -> T where F: FnOnce(u32, &mut [wl_argument]) -> T {
            match self {
                Event::Done { callback_data, } => {
                    let mut _args_array: [wl_argument; 1] = unsafe { ::std::mem::zeroed() };
                    _args_array[0].u = callback_data;
                    f(0, &mut _args_array)
                },
            }
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
}

