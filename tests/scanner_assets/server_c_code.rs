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
        #[doc(hidden)]
        __nonexhaustive,
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
        #[doc(hidden)]
        __nonexhaustive,
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
                Request::__nonexhaustive => unreachable!(),
                _ => false,
            }
        }
        fn opcode(&self) -> u16 {
            match *self {
                Request::__nonexhaustive => unreachable!(),
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
        #[doc(hidden)]
        __nonexhaustive,
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
                Event::__nonexhaustive => unreachable!(),
                _ => false,
            }
        }
        fn opcode(&self) -> u16 {
            match *self {
                Event::__nonexhaustive => unreachable!(),
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
                Event::__nonexhaustive => unreachable!(),
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
                Event::__nonexhaustive => unreachable!(),
                Event::Cake { kind, amount } => {
                    let mut _args_array: [wl_argument; 2] = unsafe { ::std::mem::zeroed() };
                    _args_array[0].u = kind.to_raw();
                    _args_array[1].u = amount;
                    f(0, &mut _args_array)
                }
            }
        }
    }
    #[derive(Clone, Eq, PartialEq)]
    pub struct WlFoo(Resource<WlFoo>);
    impl AsRef<Resource<WlFoo>> for WlFoo {
        #[inline]
        fn as_ref(&self) -> &Resource<Self> {
            &self.0
        }
    }
    impl From<Resource<WlFoo>> for WlFoo {
        #[inline]
        fn from(value: Resource<Self>) -> Self {
            WlFoo(value)
        }
    }
    impl From<WlFoo> for Resource<WlFoo> {
        #[inline]
        fn from(value: WlFoo) -> Self {
            value.0
        }
    }
    impl Interface for WlFoo {
        type Request = Request;
        type Event = Event;
        const NAME: &'static str = "wl_foo";
        const VERSION: u32 = 3;
        fn c_interface() -> *const wl_interface {
            unsafe { &super::super::c_interfaces::wl_foo_interface }
        }
    }
    impl WlFoo {
        #[doc = "a cake is possible\n\nThe server advertises that a kind of cake is available\n\nOnly available since version 2 of the interface."]
        pub fn cake(&self, kind: CakeKind, amount: u32) -> () {
            let msg = Event::Cake {
                kind: kind,
                amount: amount,
            };
            self.0.send(msg);
        }
    }
    #[doc = r" An interface for handling requests."]
    pub trait RequestHandler {
        #[doc = "do some foo\n\nThis will do some foo with its args."]
        fn foo_it(
            &mut self,
            object: WlFoo,
            number: i32,
            unumber: u32,
            text: String,
            float: f64,
            file: ::std::os::unix::io::RawFd,
        ) {
        }
        #[doc = "create a bar\n\nCreate a bar which will do its bar job."]
        fn create_bar(&mut self, object: WlFoo, id: NewResource<super::wl_bar::WlBar>) {}
    }
    impl<T: RequestHandler> HandledBy<T> for WlFoo {
        #[inline]
        fn handle(__handler: &mut T, request: Request, __object: Self) {
            match request {
                Request::FooIt {
                    number,
                    unumber,
                    text,
                    float,
                    file,
                } => __handler.foo_it(__object, number, unumber, text, float, file),
                Request::CreateBar { id } => __handler.create_bar(__object, id),
                Request::__nonexhaustive => unreachable!(),
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
            target: super::wl_foo::WlFoo,
            metadata: Vec<u8>,
            metametadata: Option<Vec<u8>>,
        },
        #[doc = "release this bar\n\nNotify the compositor that you have finished using this bar.\n\nThis is a destructor, once received this object cannot be used any longer."]
        Release,
        #[doc = "ask for erronous bindings from wayland-scanner\n\nThis request tests argument names which can break wayland-scanner.\n\nOnly available since version 2 of the interface"]
        _Self {
            _self: u32,
            _mut: u32,
            object: u32,
            ___object: u32,
            handler: u32,
            ___handler: u32,
            request: u32,
            event: u32,
        },
        #[doc(hidden)]
        __nonexhaustive,
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
            super::MessageDesc {
                name: "self",
                since: 2,
                signature: &[
                    super::ArgumentType::Uint,
                    super::ArgumentType::Uint,
                    super::ArgumentType::Uint,
                    super::ArgumentType::Uint,
                    super::ArgumentType::Uint,
                    super::ArgumentType::Uint,
                    super::ArgumentType::Uint,
                    super::ArgumentType::Uint,
                ],
            },
        ];
        type Map = super::ResourceMap;
        fn is_destructor(&self) -> bool {
            match *self {
                Request::__nonexhaustive => unreachable!(),
                Request::Release => true,
                _ => false,
            }
        }
        fn opcode(&self) -> u16 {
            match *self {
                Request::__nonexhaustive => unreachable!(),
                Request::BarDelivery { .. } => 0,
                Request::Release => 1,
                Request::_Self { .. } => 2,
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
                                map.get(val).ok_or(())?.into()
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
                2 => {
                    let mut args = msg.args.into_iter();
                    Ok(Request::_Self {
                        _self: {
                            if let Some(Argument::Uint(val)) = args.next() {
                                val
                            } else {
                                return Err(());
                            }
                        },
                        _mut: {
                            if let Some(Argument::Uint(val)) = args.next() {
                                val
                            } else {
                                return Err(());
                            }
                        },
                        object: {
                            if let Some(Argument::Uint(val)) = args.next() {
                                val
                            } else {
                                return Err(());
                            }
                        },
                        ___object: {
                            if let Some(Argument::Uint(val)) = args.next() {
                                val
                            } else {
                                return Err(());
                            }
                        },
                        handler: {
                            if let Some(Argument::Uint(val)) = args.next() {
                                val
                            } else {
                                return Err(());
                            }
                        },
                        ___handler: {
                            if let Some(Argument::Uint(val)) = args.next() {
                                val
                            } else {
                                return Err(());
                            }
                        },
                        request: {
                            if let Some(Argument::Uint(val)) = args.next() {
                                val
                            } else {
                                return Err(());
                            }
                        },
                        event: {
                            if let Some(Argument::Uint(val)) = args.next() {
                                val
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
                    let _args = ::std::slice::from_raw_parts(args, 4);
                    Ok(Request::BarDelivery {
                        kind: super::wl_foo::DeliveryKind::from_raw(_args[0].u).ok_or(())?,
                        target: Resource::<super::wl_foo::WlFoo>::from_c_ptr(_args[1].o as *mut _).into(),
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
                2 => {
                    let _args = ::std::slice::from_raw_parts(args, 8);
                    Ok(Request::_Self {
                        _self: _args[0].u,
                        _mut: _args[1].u,
                        object: _args[2].u,
                        ___object: _args[3].u,
                        handler: _args[4].u,
                        ___handler: _args[5].u,
                        request: _args[6].u,
                        event: _args[7].u,
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
        #[doc = "ask for erronous bindings from wayland-scanner\n\nThis event tests argument names which can break wayland-scanner.\n\nOnly available since version 2 of the interface"]
        _Self {
            _self: u32,
            _mut: u32,
            object: u32,
            ___object: u32,
            handler: u32,
            ___handler: u32,
            request: u32,
            event: u32,
        },
        #[doc(hidden)]
        __nonexhaustive,
    }
    impl super::MessageGroup for Event {
        const MESSAGES: &'static [super::MessageDesc] = &[super::MessageDesc {
            name: "self",
            since: 2,
            signature: &[
                super::ArgumentType::Uint,
                super::ArgumentType::Uint,
                super::ArgumentType::Uint,
                super::ArgumentType::Uint,
                super::ArgumentType::Uint,
                super::ArgumentType::Uint,
                super::ArgumentType::Uint,
                super::ArgumentType::Uint,
            ],
        }];
        type Map = super::ResourceMap;
        fn is_destructor(&self) -> bool {
            match *self {
                Event::__nonexhaustive => unreachable!(),
                _ => false,
            }
        }
        fn opcode(&self) -> u16 {
            match *self {
                Event::__nonexhaustive => unreachable!(),
                Event::_Self { .. } => 0,
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
                Event::__nonexhaustive => unreachable!(),
                Event::_Self {
                    _self,
                    _mut,
                    object,
                    ___object,
                    handler,
                    ___handler,
                    request,
                    event,
                } => Message {
                    sender_id: sender_id,
                    opcode: 0,
                    args: vec![
                        Argument::Uint(_self),
                        Argument::Uint(_mut),
                        Argument::Uint(object),
                        Argument::Uint(___object),
                        Argument::Uint(handler),
                        Argument::Uint(___handler),
                        Argument::Uint(request),
                        Argument::Uint(event),
                    ],
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
                Event::__nonexhaustive => unreachable!(),
                Event::_Self {
                    _self,
                    _mut,
                    object,
                    ___object,
                    handler,
                    ___handler,
                    request,
                    event,
                } => {
                    let mut _args_array: [wl_argument; 8] = unsafe { ::std::mem::zeroed() };
                    _args_array[0].u = _self;
                    _args_array[1].u = _mut;
                    _args_array[2].u = object;
                    _args_array[3].u = ___object;
                    _args_array[4].u = handler;
                    _args_array[5].u = ___handler;
                    _args_array[6].u = request;
                    _args_array[7].u = event;
                    f(0, &mut _args_array)
                }
            }
        }
    }
    #[derive(Clone, Eq, PartialEq)]
    pub struct WlBar(Resource<WlBar>);
    impl AsRef<Resource<WlBar>> for WlBar {
        #[inline]
        fn as_ref(&self) -> &Resource<Self> {
            &self.0
        }
    }
    impl From<Resource<WlBar>> for WlBar {
        #[inline]
        fn from(value: Resource<Self>) -> Self {
            WlBar(value)
        }
    }
    impl From<WlBar> for Resource<WlBar> {
        #[inline]
        fn from(value: WlBar) -> Self {
            value.0
        }
    }
    impl Interface for WlBar {
        type Request = Request;
        type Event = Event;
        const NAME: &'static str = "wl_bar";
        const VERSION: u32 = 1;
        fn c_interface() -> *const wl_interface {
            unsafe { &super::super::c_interfaces::wl_bar_interface }
        }
    }
    impl WlBar {
        #[doc = "ask for erronous bindings from wayland-scanner\n\nThis event tests argument names which can break wayland-scanner.\n\nOnly available since version 2 of the interface."]
        pub fn _self(
            &self,
            _self: u32,
            _mut: u32,
            object: u32,
            ___object: u32,
            handler: u32,
            ___handler: u32,
            request: u32,
            event: u32,
        ) -> () {
            let msg = Event::_Self {
                _self: _self,
                _mut: _mut,
                object: object,
                ___object: ___object,
                handler: handler,
                ___handler: ___handler,
                request: request,
                event: event,
            };
            self.0.send(msg);
        }
    }
    #[doc = r" An interface for handling requests."]
    pub trait RequestHandler {
        #[doc = "ask for a bar delivery\n\nProceed to a bar delivery of given foo.\n\nOnly available since version 2 of the interface."]
        fn bar_delivery(
            &mut self,
            object: WlBar,
            kind: super::wl_foo::DeliveryKind,
            target: super::wl_foo::WlFoo,
            metadata: Vec<u8>,
            metametadata: Option<Vec<u8>>,
        ) {
        }
        #[doc = "release this bar\n\nNotify the compositor that you have finished using this bar.\n\nThis is a destructor, you cannot send requests to this object any longer once this method is called."]
        fn release(&mut self, object: WlBar) {}
        #[doc = "ask for erronous bindings from wayland-scanner\n\nThis request tests argument names which can break wayland-scanner.\n\nOnly available since version 2 of the interface."]
        fn _self(
            &mut self,
            object: WlBar,
            _self: u32,
            _mut: u32,
            _object: u32,
            ___object: u32,
            handler: u32,
            ___handler: u32,
            request: u32,
            event: u32,
        ) {
        }
    }
    impl<T: RequestHandler> HandledBy<T> for WlBar {
        #[inline]
        fn handle(__handler: &mut T, request: Request, __object: Self) {
            match request {
                Request::BarDelivery {
                    kind,
                    target,
                    metadata,
                    metametadata,
                } => __handler.bar_delivery(__object, kind, target, metadata, metametadata),
                Request::Release {} => __handler.release(__object),
                Request::_Self {
                    _self,
                    _mut,
                    object,
                    ___object,
                    handler,
                    ___handler,
                    request,
                    event,
                } => __handler._self(
                    __object, _self, _mut, object, ___object, handler, ___handler, request, event,
                ),
                Request::__nonexhaustive => unreachable!(),
            }
        }
    }
    #[doc = r" The minimal object version supporting this request"]
    pub const REQ_BAR_DELIVERY_SINCE: u16 = 2u16;
    #[doc = r" The minimal object version supporting this request"]
    pub const REQ_RELEASE_SINCE: u16 = 1u16;
    #[doc = r" The minimal object version supporting this request"]
    pub const REQ_SELF_SINCE: u16 = 2u16;
    #[doc = r" The minimal object version supporting this event"]
    pub const EVT_SELF_SINCE: u16 = 2u16;
}
#[doc = "callback object\n\nThis object has a special behavior regarding its destructor."]
pub mod wl_callback {
    use super::sys::common::{wl_argument, wl_array, wl_interface};
    use super::sys::server::*;
    use super::{
        AnonymousObject, Argument, ArgumentType, HandledBy, Interface, Message, MessageDesc, MessageGroup,
        NewResource, Object, ObjectMetadata, Resource,
    };
    pub enum Request {
        #[doc(hidden)]
        __nonexhaustive,
    }
    impl super::MessageGroup for Request {
        const MESSAGES: &'static [super::MessageDesc] = &[];
        type Map = super::ResourceMap;
        fn is_destructor(&self) -> bool {
            match *self {
                Request::__nonexhaustive => unreachable!(),
            }
        }
        fn opcode(&self) -> u16 {
            match *self {
                Request::__nonexhaustive => unreachable!(),
            }
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
        #[doc(hidden)]
        __nonexhaustive,
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
                Event::__nonexhaustive => unreachable!(),
                Event::Done { .. } => true,
            }
        }
        fn opcode(&self) -> u16 {
            match *self {
                Event::__nonexhaustive => unreachable!(),
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
                Event::__nonexhaustive => unreachable!(),
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
                Event::__nonexhaustive => unreachable!(),
                Event::Done { callback_data } => {
                    let mut _args_array: [wl_argument; 1] = unsafe { ::std::mem::zeroed() };
                    _args_array[0].u = callback_data;
                    f(0, &mut _args_array)
                }
            }
        }
    }
    #[derive(Clone, Eq, PartialEq)]
    pub struct WlCallback(Resource<WlCallback>);
    impl AsRef<Resource<WlCallback>> for WlCallback {
        #[inline]
        fn as_ref(&self) -> &Resource<Self> {
            &self.0
        }
    }
    impl From<Resource<WlCallback>> for WlCallback {
        #[inline]
        fn from(value: Resource<Self>) -> Self {
            WlCallback(value)
        }
    }
    impl From<WlCallback> for Resource<WlCallback> {
        #[inline]
        fn from(value: WlCallback) -> Self {
            value.0
        }
    }
    impl Interface for WlCallback {
        type Request = Request;
        type Event = Event;
        const NAME: &'static str = "wl_callback";
        const VERSION: u32 = 1;
        fn c_interface() -> *const wl_interface {
            unsafe { &super::super::c_interfaces::wl_callback_interface }
        }
    }
    impl WlCallback {
        #[doc = "done event\n\nThis event is actually a destructor, but the protocol XML has no way of specifying it.\nAs such, the scanner should consider wl_callback.done as a special case.\n\nThis is a destructor, you cannot send requests to this object any longer once this method is called."]
        pub fn done(&self, callback_data: u32) -> () {
            let msg = Event::Done {
                callback_data: callback_data,
            };
            self.0.send(msg);
        }
    }
    #[doc = r" An interface for handling requests."]
    pub trait RequestHandler {}
    impl<T: RequestHandler> HandledBy<T> for WlCallback {
        #[inline]
        fn handle(__handler: &mut T, request: Request, __object: Self) {
            match request {
                Request::__nonexhaustive => unreachable!(),
            }
        }
    }
    #[doc = r" The minimal object version supporting this event"]
    pub const EVT_DONE_SINCE: u16 = 1u16;
}
