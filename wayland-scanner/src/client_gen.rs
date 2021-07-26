use proc_macro2::{Ident, Span, TokenStream};

use quote::quote;

use crate::{
    protocol::{Interface, Message, Protocol, Type},
    util::{dotted_to_relname, is_keyword, snake_to_camel},
    Side,
};

pub fn generate_client_objects(protocol: &Protocol, with_c_interfaces: bool) -> TokenStream {
    let tokens = protocol.interfaces.iter().map(generate_objects_for);
    let interfaces = crate::interfaces::generate(protocol, with_c_interfaces);
    quote!(
        pub mod __interfaces {
            #interfaces
        }
        #(#tokens)*
    )
}

fn generate_objects_for(interface: &Interface) -> TokenStream {
    let mod_name = Ident::new(&interface.name, Span::call_site());
    let mod_doc = interface.description.as_ref().map(crate::util::description_to_doc_attr);
    let iface_name = Ident::new(&snake_to_camel(&interface.name), Span::call_site());
    let iface_const_name = Ident::new(
        &format!("{}_INTERFACE", interface.name.to_ascii_uppercase()),
        Span::call_site(),
    );

    let enums = crate::common::generate_enums_for(interface);
    let sinces = crate::common::gen_since_constants(&interface.requests, &interface.events);

    let requests = crate::common::gen_message_enum(
        &Ident::new("Request", Span::call_site()),
        Side::Client,
        false,
        &interface.requests,
    );
    let events = crate::common::gen_message_enum(
        &Ident::new("Event", Span::call_site()),
        Side::Client,
        true,
        &interface.events,
    );

    let parse_body = gen_parse_body(interface);
    let write_body = gen_write_body(interface);

    quote! {
        #mod_doc
        pub mod #mod_name {
            use super::wayland_client::{backend::{smallvec, ObjectId, InvalidId, protocol::{WEnum, Argument, Message, Interface, same_interface}}, Proxy, ConnectionHandle};
            #enums
            #sinces
            #requests
            #events

            #[derive(Debug, Clone)]
            pub struct #iface_name {
                id: ObjectId
            }

            impl super::wayland_client::Proxy for #iface_name {
                type Request = Request;
                type Event = Event;

                #[inline]
                fn interface() -> &'static Interface{
                    &super::__interfaces::#iface_const_name
                }

                #[inline]
                fn id(&self) -> ObjectId {
                    self.id.clone()
                }

                #[inline]
                fn from_id(id: ObjectId) -> Result<Self, InvalidId> {
                    if same_interface(id.interface(), Self::interface()) {
                        Ok(#iface_name { id })
                    } else {
                        Err(InvalidId)
                    }
                }

                fn parse_event(msg: Message<ObjectId>) -> Result<(Self, Self::Event), Message<ObjectId>> {
                    #parse_body
                }

                fn write_request(&self, cx: &mut ConnectionHandle, request: Self::Request) -> Message<ObjectId> {
                    #write_body
                }
            }
        }
    }
}

fn gen_parse_body(interface: &Interface) -> TokenStream {
    let match_arms = interface.events.iter().enumerate().map(|(opcode, msg)| {
        let opcode = opcode as u16;
        let event_name = Ident::new(&snake_to_camel(&msg.name), Span::call_site());
        let args_pat = msg.args.iter().map(|arg| {
            let arg_name = Ident::new(
                &format!("{}{}", if is_keyword(&arg.name) { "_" } else { "" }, arg.name),
                Span::call_site(),
            );
            match arg.typ {
                Type::Uint => quote!{ Argument::Uint(#arg_name) },
                Type::Int => quote!{ Argument::Int(#arg_name) },
                Type::String => quote!{ Argument::Str(#arg_name) },
                Type::Fixed => quote!{ Argument::Fixed(#arg_name) },
                Type::Array => quote!{ Argument::Array(#arg_name) },
                Type::Object => quote!{ Argument::Object(#arg_name) },
                Type::NewId => quote!{ Argument::NewId(#arg_name) },
                Type::Fd => quote!{ Argument::Fd(#arg_name) },
                Type::Destructor => panic!("Argument {}.{}.{} has type destructor ?!", interface.name, msg.name, arg.name),
            }
        });

        let arg_names = msg.args.iter().map(|arg| {
            let arg_name = Ident::new(
                &format!("{}{}", if is_keyword(&arg.name) { "_" } else { "" }, arg.name),
                Span::call_site(),
            );
            if arg.enum_.is_some() {
                quote! { #arg_name: From::from(*#arg_name as u32) }
            } else {
                match arg.typ {
                    Type::Uint | Type::Int | Type::Fd => quote!{ #arg_name: *#arg_name },
                    Type::Fixed => quote!{ #arg_name: (*#arg_name as f64) / 256.},
                    Type::String => {
                        let string_conversion = quote! {
                            String::from_utf8_lossy(#arg_name.as_bytes()).into_owned()
                        };

                        if arg.allow_null {
                            quote! {
                                #arg_name: {
                                    let s = #string_conversion;
                                    if s.len() == 0 { None } else { Some(s) }
                                }
                            }
                        } else {
                            quote! {
                                #arg_name: #string_conversion
                            }
                        }
                    },
                    Type::Object | Type::NewId => {
                        let create_proxy = if arg.interface.is_some() {
                            quote! {
                                match Proxy::from_id(#arg_name.clone()) {
                                    Ok(p) => p,
                                    Err(_) => return Err(msg),
                                }
                            }
                        } else {
                            quote! { #arg_name.clone() }
                        };
                        if arg.allow_null {
                            quote! {
                                #arg_name: if #arg_name.is_null() { None } else { Some(#create_proxy) }
                            }
                        } else {
                            quote! {
                                #arg_name: #create_proxy
                            }
                        }
                    },
                    Type::Array => {
                        if arg.allow_null {
                            quote!(if #arg_name.len() == 0 { None } else { Some(*#arg_name.clone()) })
                        } else {
                            quote!(#arg_name: *#arg_name.clone())
                        }
                    },
                    Type::Destructor => unreachable!(),
                }
            }
        });

        quote! {
            #opcode => {
                if let [#(#args_pat),*] = &msg.args[..] {
                    Ok((me, Event::#event_name { #(#arg_names),* }))
                } else {
                    Err(msg)
                }
            }
        }
    });

    quote! {
        let me = match Self::from_id(msg.sender_id.clone()) {
            Ok(me) => me,
            Err(_) => return Err(msg),
        };
        match msg.opcode {
            #(#match_arms),*
            _ => return Err(msg)
        }
    }
}

fn gen_write_body(interface: &Interface) -> TokenStream {
    let arms = interface.requests.iter().enumerate().map(|(opcode, msg)| {
        let msg_name = Ident::new(&snake_to_camel(&msg.name), Span::call_site());
        let opcode = opcode as u16;
        let arg_names = msg.args.iter().flat_map(|arg| {
            if arg.typ == Type::NewId && arg.interface.is_some() {
                None
            } else {
                Some(Ident::new(
                    &format!("{}{}", if is_keyword(&arg.name) { "_" } else { "" }, arg.name),
                    Span::call_site(),
                ))
            }
        });
        let args = msg.args.iter().map(|arg| {
            let arg_name = Ident::new(
                &format!("{}{}", if is_keyword(&arg.name) { "_" } else { "" }, arg.name),
                Span::call_site(),
            );

            match arg.typ {
                Type::Int => if arg.enum_.is_some() { quote!{ Argument::Int(Into::<u32>::into(#arg_name) as i32) } } else { quote!{ Argument::Int(#arg_name) } },
                Type::Uint => if arg.enum_.is_some() { quote!{ Argument::Uint(#arg_name.into()) } } else { quote!{ Argument::Uint(#arg_name) } },
                Type::Fd => quote!{ Argument::Fd(#arg_name) },
                Type::Fixed => quote! { Argument::Fixed((#arg_name * 256.) as i32) },
                Type::Object => if arg.allow_null {
                    quote! { if let Some(obj) = #arg_name { Argument::Object(obj.id()) } else { Argument::Object(cx.null_id()) } }
                } else {
                    quote!{ Argument::Object(#arg_name.id()) }
                },
                Type::Array => if arg.allow_null {
                    quote! { if let Some(array) = #arg_name { Argument::Array(Box::new(array)) } else { Argument::Array(Box::new(Vec::new()))}}
                } else {
                    quote! { Argument::Array(Box::new(#arg_name)) }
                },
                Type::String => if arg.allow_null {
                    quote! { if let Some(string) = #arg_name { Argument::Str(Box::new(std::ffi::CString::new(string).unwrap())) } else { Argument::Str(Box::new(std::ffi::CString::new(Vec::new()).unwrap())) }}
                } else {
                    quote! { Argument::Str(Box::new(std::ffi::CString::new(#arg_name).unwrap())) }
                },
                Type::NewId => if arg.interface.is_some() {
                    quote! { Argument::NewId(cx.placeholder_id(None)) }
                } else {
                    quote! { Argument::Str(Box::new(std::ffi::CString::new(#arg_name.0).unwrap())), Argument::Uint(#arg_name.1), Argument::NewId(cx.placeholder_id(None)) }
                },
                Type::Destructor => panic!("Argument {}.{}.{} has type destructor ?!", interface.name, msg.name, arg.name),
            }
        });
        quote! {
            Request::#msg_name { #(#arg_names),* } => Message {
                sender_id: self.id.clone(),
                opcode: #opcode,
                args: smallvec::smallvec![
                    #(#args),*
                ]
            }
        }
    });
    quote! {
        match request {
            #(#arms),*
        }
    }
}
