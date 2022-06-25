use proc_macro2::{Ident, Literal, Span, TokenStream};

use quote::{format_ident, quote, ToTokens};

use crate::{protocol::*, util::*, Side};

pub(crate) fn generate_enums_for(interface: &Interface) -> TokenStream {
    interface.enums.iter().map(ToTokens::into_token_stream).collect()
}

impl ToTokens for Enum {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let enum_decl;
        let enum_impl;

        let doc_attr = self.description.as_ref().map(description_to_doc_attr);
        let ident = Ident::new(&snake_to_camel(&self.name), Span::call_site());

        if self.bitfield {
            let entries = self.entries.iter().map(|entry| {
                let doc_attr = entry
                    .description
                    .as_ref()
                    .map(description_to_doc_attr)
                    .or_else(|| entry.summary.as_ref().map(|s| to_doc_attr(s)));

                let prefix = if entry.name.chars().next().unwrap().is_numeric() { "_" } else { "" };
                let ident = format_ident!("{}{}", prefix, snake_to_camel(&entry.name));

                let value = Literal::u32_unsuffixed(entry.value);

                quote! {
                    #doc_attr
                    const #ident = #value;
                }
            });

            enum_decl = quote! {
                bitflags::bitflags! {
                    #doc_attr
                    pub struct #ident: u32 {
                        #(#entries)*
                    }
                }
            };
            enum_impl = quote! {
                impl std::convert::TryFrom<u32> for #ident {
                    type Error = ();
                    fn try_from(val: u32) -> Result<#ident, ()> {
                        #ident::from_bits(val).ok_or(())
                    }
                }
                impl std::convert::From<#ident> for u32 {
                    fn from(val: #ident) -> u32 {
                        val.bits()
                    }
                }
            };
        } else {
            let variants = self.entries.iter().map(|entry| {
                let doc_attr = entry
                    .description
                    .as_ref()
                    .map(description_to_doc_attr)
                    .or_else(|| entry.summary.as_ref().map(|s| to_doc_attr(s)));

                let prefix = if entry.name.chars().next().unwrap().is_numeric() { "_" } else { "" };
                let variant = format_ident!("{}{}", prefix, snake_to_camel(&entry.name));

                let value = Literal::u32_unsuffixed(entry.value);

                quote! {
                    #doc_attr
                    #variant = #value
                }
            });

            enum_decl = quote! {
                #doc_attr
                #[repr(u32)]
                #[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
                #[non_exhaustive]
                pub enum #ident {
                    #(#variants,)*
                }
            };

            let match_arms = self.entries.iter().map(|entry| {
                let value = Literal::u32_unsuffixed(entry.value);

                let prefix = if entry.name.chars().next().unwrap().is_numeric() { "_" } else { "" };
                let variant = format_ident!("{}{}", prefix, snake_to_camel(&entry.name));

                quote! {
                    #value => Ok(#ident::#variant)
                }
            });

            enum_impl = quote! {
                impl std::convert::TryFrom<u32> for #ident {
                    type Error = ();
                    fn try_from(val: u32) -> Result<#ident, ()> {
                        match val {
                            #(#match_arms,)*
                            _ => Err(())
                        }
                    }
                }
                impl std::convert::From<#ident> for u32 {
                    fn from(val: #ident) -> u32 {
                        val as u32
                    }
                }
            };
        }

        enum_decl.to_tokens(tokens);
        enum_impl.to_tokens(tokens);
    }
}

pub(crate) fn gen_since_constants(requests: &[Message], events: &[Message]) -> TokenStream {
    let req_constants = requests.iter().map(|msg| {
        let cstname = format_ident!("REQ_{}_SINCE", msg.name.to_ascii_uppercase());
        let since = msg.since;
        quote! {
            /// The minimal object version supporting this request
            pub const #cstname: u32 = #since;
        }
    });
    let evt_constants = events.iter().map(|msg| {
        let cstname = format_ident!("EVT_{}_SINCE", msg.name.to_ascii_uppercase());
        let since = msg.since;
        quote! {
            /// The minimal object version supporting this event
            pub const #cstname: u32 = #since;
        }
    });

    quote! {
        #(#req_constants)*
        #(#evt_constants)*
    }
}

pub(crate) fn gen_message_enum(
    name: &Ident,
    side: Side,
    receiver: bool,
    messages: &[Message],
) -> TokenStream {
    let variants = messages.iter().map(|msg| {
        let mut docs = String::new();
        if let Some((ref short, ref long)) = msg.description {
            docs += &format!("{}\n\n{}\n", short, long.trim());
        }
        if let Some(Type::Destructor) = msg.typ {
            docs += &format!(
                "\nThis is a destructor, once {} this object cannot be used any longer.",
                if receiver { "received" } else { "sent" },
            );
        }
        if msg.since > 1 {
            docs += &format!("\nOnly available since version {} of the interface", msg.since);
        }

        let doc_attr = to_doc_attr(&docs);
        let msg_name = Ident::new(&snake_to_camel(&msg.name), Span::call_site());
        let msg_variant_decl = if msg.args.is_empty() {
            msg_name.into_token_stream()
        } else {
            let fields = msg.args.iter().flat_map(|arg| {
                let field_name =
                    format_ident!("{}{}", if is_keyword(&arg.name) { "_" } else { "" }, arg.name);
                let field_type_inner = if let Some(ref enu) = arg.enum_ {
                    let enum_type = dotted_to_relname(enu);
                    quote! { WEnum<#enum_type> }
                } else {
                    match arg.typ {
                        Type::Uint => quote! { u32 },
                        Type::Int => quote! { i32 },
                        Type::Fixed => quote! { f64 },
                        Type::String => quote! { String },
                        Type::Array => quote! { Vec<u8> },
                        Type::Fd => quote! { ::std::os::unix::io::RawFd },
                        Type::Object => {
                            if let Some(ref iface) = arg.interface {
                                let iface_mod = Ident::new(iface, Span::call_site());
                                let iface_type =
                                    Ident::new(&snake_to_camel(iface), Span::call_site());
                                quote! { super::#iface_mod::#iface_type }
                            } else if side == Side::Client {
                                quote! { super::wayland_client::ObjectId }
                            } else {
                                quote! { super::wayland_server::ObjectId }
                            }
                        }
                        Type::NewId if !receiver && side == Side::Client => {
                            // Client-side sending does not have a pre-existing object
                            // so skip serializing it
                            if arg.interface.is_some() {
                                return None;
                            } else {
                                quote! { (&'static Interface, u32) }
                            }
                        }
                        Type::NewId => {
                            if let Some(ref iface) = arg.interface {
                                let iface_mod = Ident::new(iface, Span::call_site());
                                let iface_type =
                                    Ident::new(&snake_to_camel(iface), Span::call_site());
                                if receiver && side == Side::Server {
                                    quote! { New<super::#iface_mod::#iface_type> }
                                } else {
                                    quote! { super::#iface_mod::#iface_type }
                                }
                            } else {
                                // bind-like function
                                if side == Side::Client {
                                    quote! { (String, u32, super::wayland_client::ObjectId) }
                                } else {
                                    quote! { (String, u32, super::wayland_server::ObjectId) }
                                }
                            }
                        }
                        Type::Destructor => panic!("An argument cannot have type \"destructor\"."),
                    }
                };

                let field_type = if arg.allow_null {
                    quote! { Option<#field_type_inner> }
                } else {
                    field_type_inner.into_token_stream()
                };

                let doc_attr = arg
                    .description
                    .as_ref()
                    .map(description_to_doc_attr)
                    .or_else(|| arg.summary.as_ref().map(|s| to_doc_attr(s)));

                Some(quote! {
                    #doc_attr
                    #field_name: #field_type
                })
            });

            quote! {
                #msg_name {
                    #(#fields,)*
                }
            }
        };

        quote! {
            #doc_attr
            #msg_variant_decl
        }
    });

    quote! {
        #[derive(Debug)]
        #[non_exhaustive]
        pub enum #name {
            #(#variants,)*
        }
    }
}

pub(crate) fn gen_parse_body(interface: &Interface, side: Side) -> TokenStream {
    let msgs = match side {
        Side::Client => &interface.events,
        Side::Server => &interface.requests,
    };
    let object_type = Ident::new(
        match side {
            Side::Client => "Proxy",
            Side::Server => "Resource",
        },
        Span::call_site(),
    );
    let msg_type = Ident::new(
        match side {
            Side::Client => "Event",
            Side::Server => "Request",
        },
        Span::call_site(),
    );

    let match_arms = msgs.iter().enumerate().map(|(opcode, msg)| {
        let opcode = opcode as u16;
        let msg_name = Ident::new(&snake_to_camel(&msg.name), Span::call_site());
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
            let arg_name = format_ident!("{}{}", if is_keyword(&arg.name) { "_" } else { "" }, arg.name);
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
                    Type::Object => {
                        let create_proxy = if let Some(ref created_interface) = arg.interface {
                            let created_iface_mod = Ident::new(created_interface, Span::call_site());
                            let created_iface_type = Ident::new(&snake_to_camel(created_interface), Span::call_site());
                            quote! {
                                match <super::#created_iface_mod::#created_iface_type as #object_type>::from_id(conn, #arg_name.clone()) {
                                    Ok(p) => p,
                                    Err(_) => return Err(DispatchError::BadMessage { msg, interface: Self::interface().name }),
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
                    Type::NewId => {
                        let create_proxy = if let Some(ref created_interface) = arg.interface {
                            let created_iface_mod = Ident::new(created_interface, Span::call_site());
                            let created_iface_type = Ident::new(&snake_to_camel(created_interface), Span::call_site());
                            quote! {
                                match <super::#created_iface_mod::#created_iface_type as #object_type>::from_id(conn, #arg_name.clone()) {
                                    Ok(p) => p,
                                    Err(_) => return Err(DispatchError::BadMessage { msg, interface: Self::interface().name }),
                                }
                            }
                        } else if side == Side::Server {
                            quote! { New::wrap(#arg_name.clone()) }
                        } else {
                            quote! { #arg_name.clone() }
                        };
                        if arg.allow_null {
                            if side == Side::Server {
                                quote! {
                                    #arg_name: if #arg_name.is_null() { None } else { Some(New::wrap(#create_proxy)) }
                                }
                            } else {
                                quote! {
                                    #arg_name: if #arg_name.is_null() { None } else { Some(#create_proxy) }
                                }
                            }
                        } else if side == Side::Server {
                            quote! {
                                #arg_name: New::wrap(#create_proxy)
                            }
                        } else  {
                            quote! {
                                #arg_name: #create_proxy
                            }
                        }
                    },
                    Type::Array => {
                        if arg.allow_null {
                            quote! { if #arg_name.len() == 0 { None } else { Some(*#arg_name.clone()) } }
                        } else {
                            quote! { #arg_name: *#arg_name.clone() }
                        }
                    },
                    Type::Destructor => unreachable!(),
                }
            }
        });

        quote! {
            #opcode => {
                if let [#(#args_pat),*] = &msg.args[..] {
                    Ok((me, #msg_type::#msg_name { #(#arg_names),* }))
                } else {
                    Err(DispatchError::BadMessage { msg, interface: Self::interface().name })
                }
            }
        }
    });

    quote! {
        let me = Self::from_id(conn, msg.sender_id.clone()).unwrap();
        match msg.opcode {
            #(#match_arms),*
            _ => Err(DispatchError::BadMessage { msg, interface: Self::interface().name }),
        }
    }
}

pub(crate) fn gen_write_body(interface: &Interface, side: Side) -> TokenStream {
    let msgs = match side {
        Side::Client => &interface.requests,
        Side::Server => &interface.events,
    };
    let msg_type = Ident::new(
        match side {
            Side::Client => "Request",
            Side::Server => "Event",
        },
        Span::call_site(),
    );
    let arms = msgs.iter().enumerate().map(|(opcode, msg)| {
        let msg_name = Ident::new(&snake_to_camel(&msg.name), Span::call_site());
        let opcode = opcode as u16;
        let arg_names = msg.args.iter().flat_map(|arg| {
            if arg.typ == Type::NewId && arg.interface.is_some() && side == Side::Client {
                None
            } else {
                Some(format_ident!("{}{}", if is_keyword(&arg.name) { "_" } else { "" }, arg.name))
            }
        });
        let args = msg.args.iter().map(|arg| {
            let arg_name = format_ident!("{}{}", if is_keyword(&arg.name) { "_" } else { "" }, arg.name);

            match arg.typ {
                Type::Int => if arg.enum_.is_some() { quote!{ Argument::Int(Into::<u32>::into(#arg_name) as i32) } } else { quote!{ Argument::Int(#arg_name) } },
                Type::Uint => if arg.enum_.is_some() { quote!{ Argument::Uint(#arg_name.into()) } } else { quote!{ Argument::Uint(#arg_name) } },
                Type::Fd => quote!{ Argument::Fd(#arg_name) },
                Type::Fixed => quote! { Argument::Fixed((#arg_name * 256.) as i32) },
                Type::Object => if arg.allow_null {
                    if side == Side::Server {
                        quote! { if let Some(obj) = #arg_name { Argument::Object(Resource::id(&obj)) } else { Argument::Object(conn.null_id()) } }
                    } else {
                        quote! { if let Some(obj) = #arg_name { Argument::Object(Proxy::id(&obj)) } else { Argument::Object(Connection::null_id()) } }
                    }
                } else if side == Side::Server {
                    quote!{ Argument::Object(Resource::id(&#arg_name)) }
                } else {
                    quote!{ Argument::Object(Proxy::id(&#arg_name)) }
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
                Type::NewId => if side == Side::Client {
                    if let Some(ref created_interface) = arg.interface {
                        let created_iface_mod = Ident::new(created_interface, Span::call_site());
                        let created_iface_type = Ident::new(&snake_to_camel(created_interface), Span::call_site());
                        quote! { {
                            let my_info = conn.object_info(self.id())?;
                            child_spec = Some((super::#created_iface_mod::#created_iface_type::interface(), my_info.version));
                            Argument::NewId(Connection::null_id())
                        } }
                    } else {
                        quote! {
                            Argument::Str(Box::new(std::ffi::CString::new(#arg_name.0.name).unwrap())),
                            Argument::Uint(#arg_name.1),
                            {
                                child_spec = Some((#arg_name.0, #arg_name.1));
                                Argument::NewId(Connection::null_id())
                            }
                        }
                    }
                } else {
                    // server-side NewId is the same as Object
                    if arg.allow_null {
                        quote! { if let Some(obj) = #arg_name { Argument::NewId(Resource::id(&obj)) } else { Argument::NewId(conn.null_id()) } }
                    } else {
                        quote!{ Argument::NewId(Resource::id(&#arg_name)) }
                    }
                },
                Type::Destructor => panic!("Argument {}.{}.{} has type destructor ?!", interface.name, msg.name, arg.name),
            }
        });
        if side == Side::Client {
            quote! {
                #msg_type::#msg_name { #(#arg_names),* } => {
                    let mut child_spec = None;
                    let args = smallvec::smallvec![
                        #(#args),*
                    ];
                    Ok((Message {
                        sender_id: self.id.clone(),
                        opcode: #opcode,
                        args
                    }, child_spec))
                }
            }
        } else {
            quote! {
                #msg_type::#msg_name { #(#arg_names),* } => Ok(Message {
                    sender_id: self.id.clone(),
                    opcode: #opcode,
                    args: smallvec::smallvec![
                        #(#args),*
                    ]
                })
            }
        }
    });
    quote! {
        match msg {
            #(#arms),*
        }
    }
}
