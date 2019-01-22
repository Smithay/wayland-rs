#[doc = "Interface for fooing\n\nThis is the dedicated interface for doing foos over any\nkind of other foos."]
pub mod wl_foo {
    use super::sys::common::{wl_argument, wl_array, wl_interface};
    use super::sys::server::*;
    use super::{
        AnonymousObject, Argument, ArgumentType, HandledBy, Interface, Message, MessageDesc, MessageGroup,
        NewResource, Object, ObjectMetadata, Resource,
    };
    #[doc = "Possible cake kinds\n\nList of the possible kind of cake supported by the protocol."]
    #[repr(u32)]
    #[derive(Copy, Clone, Debug, PartialEq)]
    pub enum CakeKind {
        #[doc = "mild cake without much flavor"]
        Basic = 0,
        #[doc = "spicy cake to burn your tongue"]
        Spicy = 1,
        #[doc = "fruity cake to get vitamins"]
        Fruity = 2,
    }
    impl CakeKind {
        pub fn from_raw(n: u32) -> Option<CakeKind> {
            match n {
                0 => Some(CakeKind::Basic),
                1 => Some(CakeKind::Spicy),
                2 => Some(CakeKind::Fruity),
                _ => Option::None,
            }
        }
        pub fn to_raw(&self) -> u32 {
            *self as u32
        }
    }
    bitflags! { # [ doc = "possible delivery modes" ] pub struct DeliveryKind : u32 { # [ doc = "pick your cake up yourself" ] const PickUp = 1 ; # [ doc = "flying drone delivery" ] const Drone = 2 ; # [ doc = "because we fear nothing" ] const Catapult = 4 ; } }
    impl DeliveryKind {
        pub fn from_raw(n: u32) -> Option<DeliveryKind> {
            Some(DeliveryKind::from_bits_truncate(n))
        }
        pub fn to_raw(&self) -> u32 {
            self.bits()
        }
    }
    pub enum Request {
        #[doc = "do some foo\n\nThis will do some foo with its args."]
        FooIt {
            number: i32,
            unumber: u32,
            text: String,
            float: f64,
            file: ::std::os::unix::io::RawFd,
        },
        #[doc = "create a bar\n\nCreate a bar which will do its bar job."]
        CreateBar { id: NewResource<super::wl_bar::WlBar> },
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
                ],
            },
            super::MessageDesc {
                name: "create_bar",
                since: 1,
                signature: &[super::ArgumentType::NewId],
            },
        ];
        type Map = super::ResourceMap;
        fn is_destructor(&self) -> bool {
            match *self {
                _ => false,
            }
        }
        fn opcode(&self) -> u16 {
            match *self {
                Request::FooIt { .. } => 0,
                Request::CreateBar { .. } => 1,
            }
        }
        fn child<Meta: ObjectMetadata>(opcode: u16, version: u32, meta: &Meta) -> Option<Object<Meta>> {
            match opcode {
                1 => Some(Object::from_interface::<super::wl_bar::WlBar>(
                    version,
                    meta.child(),
                )),
                _ => None,
            }
        }
        fn from_raw(msg: Message, map: &mut Self::Map) -> Result<Self, ()> {
            match msg.opcode {
                0 => {
                    let mut args = msg.args.into_iter();
                    Ok(Request::FooIt {
                        number: {
                            if let Some(Argument::Int(val)) = args.next() {
                                val
                            } else {
                                return Err(());
                            }
                        },
                        unumber: {
                            if let Some(Argument::Uint(val)) = args.next() {
                                val
                            } else {
                                return Err(());
                            }
                        },
                        text: {
                            if let Some(Argument::Str(val)) = args.next() {
                                let s = String::from_utf8(val.into_bytes())
                                    .unwrap_or_else(|e| String::from_utf8_lossy(&e.into_bytes()).into());
                                s
                            } else {
                                return Err(());
                            }
                        },
                        float: {
                            if let Some(Argument::Fixed(val)) = args.next() {
                                (val as f64) / 256.
                            } else {
                                return Err(());
                            }
                        },
                        file: {
                            if let Some(Argument::Fd(val)) = args.next() {
                                val
                            } else {
                                return Err(());
                            }
                        },
                    })
                }
                1 => {
                    let mut args = msg.args.into_iter();
                    Ok(Request::CreateBar {
                        id: {
                            if let Some(Argument::NewId(val)) = args.next() {
                                map.get_new(val).ok_or(())?
                            } else {
                                return Err(());
                            }
                        },
                    })
                }
                _ => Err(()),
            }
        }
        fn into_raw(self, sender_id: u32) -> Message {
            panic!("Request::into_raw can not be used Server-side.")
        }
        unsafe fn from_raw_c(
            obj: *mut ::std::os::raw::c_void,
            opcode: u32,
            args: *const wl_argument,
        ) -> Result<Request, ()> {
            match opcode {
                0 => {
                    let _args = ::std::slice::from_raw_parts(args, 5);
                    Ok(Request::FooIt {
                        number: _args[0].i,
                        unumber: _args[1].u,
                        text: ::std::ffi::CStr::from_ptr(_args[2].s)
                            .to_string_lossy()
                            .into_owned(),
                        float: (_args[3].f as f64) / 256.,
                        file: _args[4].h,
                    })
                }
                1 => {
                    let _args = ::std::slice::from_raw_parts(args, 1);
                    Ok(Request::CreateBar {
                        id: {
                            let client =
                                ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_resource_get_client, obj as *mut _);
                            let version =
                                ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_resource_get_version, obj as *mut _);
                            let new_ptr = ffi_dispatch!(
                                WAYLAND_SERVER_HANDLE,
                                wl_resource_create,
                                client,
                                super::wl_bar::WlBar::c_interface(),
                                version,
                                _args[0].n
                            );
                            NewResource::<super::wl_bar::WlBar>::from_c_ptr(new_ptr)
                        },
                    })
                }
                _ => return Err(()),
            }
        }
        fn as_raw_c_in<F, T>(self, f: F) -> T
        where
            F: FnOnce(u32, &mut [wl_argument]) -> T,
        {
            panic!("Request::as_raw_c_in can not be used Server-side.")
        }
    }
    pub enum Event {
        #[doc = "a cake is possible\n\nThe server advertises that a kind of cake is available\n\nOnly available since version 2 of the interface"]
        Cake { kind: CakeKind, amount: u32 },
    }
    impl super::MessageGroup for Event {
        const MESSAGES: &'static [super::MessageDesc] = &[super::MessageDesc {
            name: "cake",
            since: 2,
            signature: &[super::ArgumentType::Uint, super::ArgumentType::Uint],
        }];
        type Map = super::ResourceMap;
        fn is_destructor(&self) -> bool {
            match *self {
                _ => false,
            }
        }
        fn opcode(&self) -> u16 {
            match *self {
                Event::Cake { .. } => 0,
            }
        }
        fn child<Meta: ObjectMetadata>(opcode: u16, version: u32, meta: &Meta) -> Option<Object<Meta>> {
            match opcode {
                _ => None,
            }
        }
        fn from_raw(msg: Message, map: &mut Self::Map) -> Result<Self, ()> {
            panic!("Event::from_raw can not be used Server-side.")
        }
        fn into_raw(self, sender_id: u32) -> Message {
            match self {
                Event::Cake { kind, amount } => Message {
                    sender_id: sender_id,
                    opcode: 0,
                    args: vec![Argument::Uint(kind.to_raw()), Argument::Uint(amount)],
                },
            }
        }
        unsafe fn from_raw_c(
            obj: *mut ::std::os::raw::c_void,
            opcode: u32,
            args: *const wl_argument,
        ) -> Result<Event, ()> {
            panic!("Event::from_raw_c can not be used Server-side.")
        }
        fn as_raw_c_in<F, T>(self, f: F) -> T
        where
            F: FnOnce(u32, &mut [wl_argument]) -> T,
        {
            match self {
                Event::Cake { kind, amount } => {
                    let mut _args_array: [wl_argument; 2] = unsafe { ::std::mem::zeroed() };
                    _args_array[0].u = kind.to_raw();
                    _args_array[1].u = amount;
                    f(0, &mut _args_array)
                }
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
    #[doc = r" An interface for handling requests."]
    pub trait RequestHandler {
        #[doc = "do some foo\n\nThis will do some foo with its args."]
        fn foo_it(
            &mut self,
            request: Resource<WlFoo>,
            number: i32,
            unumber: u32,
            text: String,
            float: f64,
            file: ::std::os::unix::io::RawFd,
        ) {
        }
        #[doc = "create a bar\n\nCreate a bar which will do its bar job."]
        fn create_bar(&mut self, request: Resource<WlFoo>, id: NewResource<super::wl_bar::WlBar>) {}
    }
    impl<T: RequestHandler> HandledBy<T> for WlFoo {
        #[inline]
        fn handle(handler: &mut T, request: Request, resource: Resource<Self>) {
            match request {
                Request::FooIt {
                    number,
                    unumber,
                    text,
                    float,
                    file,
                } => handler.foo_it(resource, number, unumber, text, float, file),
                Request::CreateBar { id } => handler.create_bar(resource, id),
            }
        }
    }
    #[doc = r" The minimal object version supporting this request"]
    pub const REQ_FOO_IT_SINCE: u16 = 1u16;
    #[doc = r" The minimal object version supporting this request"]
    pub const REQ_CREATE_BAR_SINCE: u16 = 1u16;
    #[doc = r" The minimal object version supporting this event"]
    pub const EVT_CAKE_SINCE: u16 = 2u16;
}
#[doc = "Interface for bars\n\nThis interface allows you to bar your foos."]
pub mod wl_bar {
    use super::sys::common::{wl_argument, wl_array, wl_interface};
    use super::sys::server::*;
    use super::{
        AnonymousObject, Argument, ArgumentType, HandledBy, Interface, Message, MessageDesc, MessageGroup,
        NewResource, Object, ObjectMetadata, Resource,
    };
    pub enum Request {
        #[doc = "ask for a bar delivery\n\nProceed to a bar delivery of given foo.\n\nOnly available since version 2 of the interface"]
        BarDelivery {
            kind: super::wl_foo::DeliveryKind,
            target: Resource<super::wl_foo::WlFoo>,
            metadata: Vec<u8>,
            metametadata: Option<Vec<u8>>,
        },
        #[doc = "release this bar\n\nNotify the compositor that you have finished using this bar.\n\nThis is a destructor, once received this object cannot be used any longer."]
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
                    super::ArgumentType::Array,
                ],
            },
            super::MessageDesc {
                name: "release",
                since: 1,
                signature: &[],
            },
        ];
        type Map = super::ResourceMap;
        fn is_destructor(&self) -> bool {
            match *self {
                Request::Release => true,
                _ => false,
            }
        }
        fn opcode(&self) -> u16 {
            match *self {
                Request::BarDelivery { .. } => 0,
                Request::Release => 1,
            }
        }
        fn child<Meta: ObjectMetadata>(opcode: u16, version: u32, meta: &Meta) -> Option<Object<Meta>> {
            match opcode {
                _ => None,
            }
        }
        fn from_raw(msg: Message, map: &mut Self::Map) -> Result<Self, ()> {
            match msg.opcode {
                0 => {
                    let mut args = msg.args.into_iter();
                    Ok(Request::BarDelivery {
                        kind: {
                            if let Some(Argument::Uint(val)) = args.next() {
                                super::wl_foo::DeliveryKind::from_raw(val).ok_or(())?
                            } else {
                                return Err(());
                            }
                        },
                        target: {
                            if let Some(Argument::Object(val)) = args.next() {
                                map.get(val).ok_or(())?
                            } else {
                                return Err(());
                            }
                        },
                        metadata: {
                            if let Some(Argument::Array(val)) = args.next() {
                                val
                            } else {
                                return Err(());
                            }
                        },
                        metametadata: {
                            if let Some(Argument::Array(val)) = args.next() {
                                if val.len() == 0 {
                                    None
                                } else {
                                    Some(val)
                                }
                            } else {
                                return Err(());
                            }
                        },
                    })
                }
                1 => Ok(Request::Release),
                _ => Err(()),
            }
        }
        fn into_raw(self, sender_id: u32) -> Message {
            panic!("Request::into_raw can not be used Server-side.")
        }
        unsafe fn from_raw_c(
            obj: *mut ::std::os::raw::c_void,
            opcode: u32,
            args: *const wl_argument,
        ) -> Result<Request, ()> {
            match opcode {
                0 => {
                    let _args = ::std::slice::from_raw_parts(args, 4);
                    Ok(Request::BarDelivery {
                        kind: super::wl_foo::DeliveryKind::from_raw(_args[0].u).ok_or(())?,
                        target: Resource::<super::wl_foo::WlFoo>::from_c_ptr(_args[1].o as *mut _),
                        metadata: {
                            let array = &*_args[2].a;
                            ::std::slice::from_raw_parts(array.data as *const u8, array.size).to_owned()
                        },
                        metametadata: if _args[3].a.is_null() {
                            None
                        } else {
                            Some({
                                let array = &*_args[3].a;
                                ::std::slice::from_raw_parts(array.data as *const u8, array.size).to_owned()
                            })
                        },
                    })
                }
                1 => Ok(Request::Release),
                _ => return Err(()),
            }
        }
        fn as_raw_c_in<F, T>(self, f: F) -> T
        where
            F: FnOnce(u32, &mut [wl_argument]) -> T,
        {
            panic!("Request::as_raw_c_in can not be used Server-side.")
        }
    }
    pub enum Event {}
    impl super::MessageGroup for Event {
        const MESSAGES: &'static [super::MessageDesc] = &[];
        type Map = super::ResourceMap;
        fn is_destructor(&self) -> bool {
            match *self {}
        }
        fn opcode(&self) -> u16 {
            match *self {}
        }
        fn child<Meta: ObjectMetadata>(opcode: u16, version: u32, meta: &Meta) -> Option<Object<Meta>> {
            match opcode {
                _ => None,
            }
        }
        fn from_raw(msg: Message, map: &mut Self::Map) -> Result<Self, ()> {
            panic!("Event::from_raw can not be used Server-side.")
        }
        fn into_raw(self, sender_id: u32) -> Message {
            match self {}
        }
        unsafe fn from_raw_c(
            obj: *mut ::std::os::raw::c_void,
            opcode: u32,
            args: *const wl_argument,
        ) -> Result<Event, ()> {
            panic!("Event::from_raw_c can not be used Server-side.")
        }
        fn as_raw_c_in<F, T>(self, f: F) -> T
        where
            F: FnOnce(u32, &mut [wl_argument]) -> T,
        {
            match self {}
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
    #[doc = r" An interface for handling requests."]
    pub trait RequestHandler {
        #[doc = "ask for a bar delivery\n\nProceed to a bar delivery of given foo.\n\nOnly available since version 2 of the interface."]
        fn bar_delivery(
            &mut self,
            request: Resource<WlBar>,
            kind: super::wl_foo::DeliveryKind,
            target: Resource<super::wl_foo::WlFoo>,
            metadata: Vec<u8>,
            metametadata: Option<Vec<u8>>,
        ) {
        }
        #[doc = "release this bar\n\nNotify the compositor that you have finished using this bar.\n\nThis is a destructor, you cannot send requests to this object any longer once this method is called."]
        fn release(&mut self, request: Resource<WlBar>) {}
    }
    impl<T: RequestHandler> HandledBy<T> for WlBar {
        #[inline]
        fn handle(handler: &mut T, request: Request, resource: Resource<Self>) {
            match request {
                Request::BarDelivery {
                    kind,
                    target,
                    metadata,
                    metametadata,
                } => handler.bar_delivery(resource, kind, target, metadata, metametadata),
                Request::Release {} => handler.release(resource),
            }
        }
    }
    #[doc = r" The minimal object version supporting this request"]
    pub const REQ_BAR_DELIVERY_SINCE: u16 = 2u16;
    #[doc = r" The minimal object version supporting this request"]
    pub const REQ_RELEASE_SINCE: u16 = 1u16;
}
#[doc = "callback object\n\nThis object has a special behavior regarding its destructor."]
pub mod wl_callback {
    use super::sys::common::{wl_argument, wl_array, wl_interface};
    use super::sys::server::*;
    use super::{
        AnonymousObject, Argument, ArgumentType, HandledBy, Interface, Message, MessageDesc, MessageGroup,
        NewResource, Object, ObjectMetadata, Resource,
    };
    pub enum Request {}
    impl super::MessageGroup for Request {
        const MESSAGES: &'static [super::MessageDesc] = &[];
        type Map = super::ResourceMap;
        fn is_destructor(&self) -> bool {
            match *self {}
        }
        fn opcode(&self) -> u16 {
            match *self {}
        }
        fn child<Meta: ObjectMetadata>(opcode: u16, version: u32, meta: &Meta) -> Option<Object<Meta>> {
            match opcode {
                _ => None,
            }
        }
        fn from_raw(msg: Message, map: &mut Self::Map) -> Result<Self, ()> {
            match msg.opcode {
                _ => Err(()),
            }
        }
        fn into_raw(self, sender_id: u32) -> Message {
            panic!("Request::into_raw can not be used Server-side.")
        }
        unsafe fn from_raw_c(
            obj: *mut ::std::os::raw::c_void,
            opcode: u32,
            args: *const wl_argument,
        ) -> Result<Request, ()> {
            match opcode {
                _ => return Err(()),
            }
        }
        fn as_raw_c_in<F, T>(self, f: F) -> T
        where
            F: FnOnce(u32, &mut [wl_argument]) -> T,
        {
            panic!("Request::as_raw_c_in can not be used Server-side.")
        }
    }
    pub enum Event {
        #[doc = "done event\n\nThis event is actually a destructor, but the protocol XML has no way of specifying it.\nAs such, the scanner should consider wl_callback.done as a special case.\n\nThis is a destructor, once sent this object cannot be used any longer."]
        Done { callback_data: u32 },
    }
    impl super::MessageGroup for Event {
        const MESSAGES: &'static [super::MessageDesc] = &[super::MessageDesc {
            name: "done",
            since: 1,
            signature: &[super::ArgumentType::Uint],
        }];
        type Map = super::ResourceMap;
        fn is_destructor(&self) -> bool {
            match *self {
                Event::Done { .. } => true,
            }
        }
        fn opcode(&self) -> u16 {
            match *self {
                Event::Done { .. } => 0,
            }
        }
        fn child<Meta: ObjectMetadata>(opcode: u16, version: u32, meta: &Meta) -> Option<Object<Meta>> {
            match opcode {
                _ => None,
            }
        }
        fn from_raw(msg: Message, map: &mut Self::Map) -> Result<Self, ()> {
            panic!("Event::from_raw can not be used Server-side.")
        }
        fn into_raw(self, sender_id: u32) -> Message {
            match self {
                Event::Done { callback_data } => Message {
                    sender_id: sender_id,
                    opcode: 0,
                    args: vec![Argument::Uint(callback_data)],
                },
            }
        }
        unsafe fn from_raw_c(
            obj: *mut ::std::os::raw::c_void,
            opcode: u32,
            args: *const wl_argument,
        ) -> Result<Event, ()> {
            panic!("Event::from_raw_c can not be used Server-side.")
        }
        fn as_raw_c_in<F, T>(self, f: F) -> T
        where
            F: FnOnce(u32, &mut [wl_argument]) -> T,
        {
            match self {
                Event::Done { callback_data } => {
                    let mut _args_array: [wl_argument; 1] = unsafe { ::std::mem::zeroed() };
                    _args_array[0].u = callback_data;
                    f(0, &mut _args_array)
                }
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
    #[doc = r" An interface for handling requests."]
    pub trait RequestHandler {}
    impl<T: RequestHandler> HandledBy<T> for WlCallback {
        #[inline]
        fn handle(handler: &mut T, request: Request, resource: Resource<Self>) {
            match request {}
        }
    }
    #[doc = r" The minimal object version supporting this event"]
    pub const EVT_DONE_SINCE: u16 = 1u16;
}
