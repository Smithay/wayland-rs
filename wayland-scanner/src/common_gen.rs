use std::iter;

use proc_macro2::{Ident, Literal, Span, TokenStream};
use quote::ToTokens;

use protocol::*;
use util::*;
use Side;

pub(crate) fn to_doc_attr(text: &str) -> TokenStream {
    let text = text
        .lines()
        .map(|line| line.trim())
        .collect::<Vec<_>>()
        .join("\n");
    let text = text.trim();

    quote!(#[doc = #text])
}

pub(crate) fn description_to_doc_attr(&(ref short, ref long): &(String, String)) -> TokenStream {
    to_doc_attr(&format!("{}\n\n{}", short, long))
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

                let prefix = if entry.name.chars().next().unwrap().is_numeric() {
                    "_"
                } else {
                    ""
                };
                let ident = Ident::new(
                    &format!("{}{}", prefix, snake_to_camel(&entry.name)),
                    Span::call_site(),
                );

                let value = Literal::u32_unsuffixed(entry.value);

                quote! {
                    #doc_attr
                    const #ident = #value;
                }
            });

            enum_decl = quote! {
                bitflags! {
                    #doc_attr
                    pub struct #ident: u32 {
                        #(#entries)*
                    }
                }
            };
            enum_impl = quote! {
                impl #ident {
                    pub fn from_raw(n: u32) -> Option<#ident> {
                        Some(#ident::from_bits_truncate(n))
                    }

                    pub fn to_raw(&self) -> u32 {
                        self.bits()
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

                let prefix = if entry.name.chars().next().unwrap().is_numeric() {
                    "_"
                } else {
                    ""
                };
                let variant = Ident::new(
                    &format!("{}{}", prefix, snake_to_camel(&entry.name)),
                    Span::call_site(),
                );

                let value = Literal::u32_unsuffixed(entry.value);

                quote! {
                    #doc_attr
                    #variant = #value
                }
            });

            enum_decl = quote! {
                #doc_attr
                #[repr(u32)]
                #[derive(Copy, Clone, Debug, PartialEq)]
                pub enum #ident {
                    #(#variants,)*
                }
            };

            let match_arms = self.entries.iter().map(|entry| {
                let value = Literal::u32_unsuffixed(entry.value);

                let prefix = if entry.name.chars().next().unwrap().is_numeric() {
                    "_"
                } else {
                    ""
                };
                let variant = Ident::new(
                    &format!("{}{}", prefix, snake_to_camel(&entry.name)),
                    Span::call_site(),
                );

                quote! {
                    #value => Some(#ident::#variant)
                }
            });

            enum_impl = quote! {
                impl #ident {
                    pub fn from_raw(n: u32) -> Option<#ident> {
                        match n {
                            #(#match_arms,)*
                            _ => Option::None
                        }
                    }

                    pub fn to_raw(&self) -> u32 {
                        *self as u32
                    }
                }
            };
        }

        enum_decl.to_tokens(tokens);
        enum_impl.to_tokens(tokens);
    }
}

pub(crate) fn gen_messagegroup(
    name: &Ident,
    side: Side,
    receiver: bool,
    messages: &[Message],
    addon: Option<TokenStream>,
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
            let fields = msg.args.iter().map(|arg| {
                let field_name = Ident::new(&arg.name, Span::call_site());
                let field_type_inner = if let Some(ref enu) = arg.enum_ {
                    dotted_to_relname(enu)
                } else {
                    match arg.typ {
                        Type::Uint => quote!(u32),
                        Type::Int => quote!(i32),
                        Type::Fixed => quote!(f64),
                        Type::String => quote!(String),
                        Type::Array => quote!(Vec<u8>),
                        Type::Fd => quote!(::std::os::unix::io::RawFd),
                        Type::Object => {
                            let object_name = side.object_name();
                            if let Some(ref iface) = arg.interface {
                                let iface_mod = Ident::new(&iface, Span::call_site());
                                let iface_type = Ident::new(&snake_to_camel(iface), Span::call_site());
                                quote!(#object_name<super::#iface_mod::#iface_type>)
                            } else {
                                quote!(#object_name<AnonymousObject>)
                            }
                        }
                        Type::NewId => {
                            let prefix = if receiver { "New" } else { "" };
                            let object_name =
                                Ident::new(&format!("{}{}", prefix, side.object_name()), Span::call_site());
                            if let Some(ref iface) = arg.interface {
                                let iface_mod = Ident::new(&iface, Span::call_site());
                                let iface_type = Ident::new(&snake_to_camel(iface), Span::call_site());
                                quote!(#object_name<super::#iface_mod::#iface_type>)
                            } else {
                                // bind-like function
                                quote!((String, u32, #object_name<AnonymousObject>))
                            }
                        }
                        Type::Destructor => panic!("An argument cannot have type \"destructor\"."),
                    }
                };

                let field_type = if arg.allow_null {
                    quote!(Option<#field_type_inner>)
                } else {
                    field_type_inner.into_token_stream()
                };

                quote! {
                    #field_name: #field_type
                }
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

    let message_array_values = messages.iter().map(|msg| {
        let name_value = &msg.name;
        let since_value = Literal::u16_unsuffixed(msg.since);
        let signature_values = msg.args.iter().map(|arg| {
            let common_type = arg.typ.common_type();
            quote!(super::ArgumentType::#common_type)
        });

        quote! {
            super::MessageDesc {
                name: #name_value,
                since: #since_value,
                signature: &[
                    #(#signature_values,)*
                ],
            }
        }
    });

    let map_type = if side == Side::Client {
        quote!(ProxyMap)
    } else {
        quote!(ResourceMap)
    };

    // Can't be a closure because closures are never Copy / Clone in rustc < 1.26.0, and we supports 1.21.0
    fn map_fn((ref msg, ref name): (&Message, &Ident)) -> TokenStream {
        let msg_type = Ident::new(&snake_to_camel(&msg.name), Span::call_site());
        let msg_type_qualified = quote!(#name::#msg_type);

        if msg.args.is_empty() {
            msg_type_qualified
        } else {
            quote!(#msg_type_qualified { .. })
        }
    };

    let message_match_patterns = messages.iter().zip(iter::repeat(name)).map(map_fn);

    let mut is_destructor_match_arms = messages
        .iter()
        .zip(message_match_patterns.clone())
        .filter(|&(msg, _)| msg.typ == Some(Type::Destructor))
        .map(|(_, pattern)| quote!(#pattern => true))
        .collect::<Vec<_>>();

    if messages.len() > is_destructor_match_arms.len() {
        is_destructor_match_arms.push(quote!(_ => false));
    }

    let opcode_match_arms = message_match_patterns.enumerate().map(|(opcode, pattern)| {
        let value = Literal::u16_unsuffixed(opcode as u16);
        quote!(#pattern => #value)
    });

    let child_match_arms = messages
        .iter()
        .enumerate()
        .filter_map(|(opcode, msg)| {
            let mut it = msg.args.iter().filter_map(|a| {
                if a.typ == Type::NewId {
                    a.interface.as_ref()
                } else {
                    None
                }
            });

            it.next().map(|new_iface| {
                assert!(
                    it.next().is_none(),
                    "Got a message with more than one new_id in {}.{}",
                    name,
                    msg.name
                );

                let pattern = Literal::u16_unsuffixed(opcode as u16);
                let new_iface_mod = Ident::new(new_iface, Span::call_site());
                let new_iface_type = Ident::new(&snake_to_camel(new_iface), Span::call_site());

                quote! {
                    #pattern => Some(Object::from_interface::<super::#new_iface_mod::#new_iface_type>(
                        version,
                        meta.child(),
                    ))
                }
            })
        })
        .chain(iter::once(quote!(_ => None)));

    let from_raw_body = if receiver {
        let match_arms = messages
            .iter()
            .enumerate()
            .map(|(opcode, msg)| {
                let pattern = Literal::u16_unsuffixed(opcode as u16);
                let msg_type = Ident::new(&snake_to_camel(&msg.name), Span::call_site());
                let msg_type_qualified = quote!(#name::#msg_type);

                let block = if msg.args.is_empty() {
                    quote!(Ok(#msg_type_qualified))
                } else {
                    let fields = msg.args.iter().map(|arg| {
                        let field_name = Ident::new(&arg.name, Span::call_site());
                        let some_code_path = match arg.typ {
                            Type::Int => {
                                if let Some(ref enu) = arg.enum_ {
                                    let enum_ident = dotted_to_relname(enu);
                                    quote!(#enum_ident::from_raw(val as u32).ok_or(())?)
                                } else {
                                    quote!(val)
                                }
                            }
                            Type::Uint => {
                                if let Some(ref enu) = arg.enum_ {
                                    let enum_ident = dotted_to_relname(enu);
                                    quote!(#enum_ident::from_raw(val).ok_or(())?)
                                } else {
                                    quote!(val)
                                }
                            }
                            Type::Fixed => quote!((val as f64) / 256.),
                            Type::Array => {
                                if arg.allow_null {
                                    quote!(if val.len() == 0 { None } else { Some(val) })
                                } else {
                                    quote!(val)
                                }
                            }
                            Type::String => {
                                let string_conversion = quote! {
                                    let s = String::from_utf8(val.into_bytes())
                                        .unwrap_or_else(|e| String::from_utf8_lossy(&e.into_bytes()).into());
                                };

                                if arg.allow_null {
                                    quote! {
                                        #string_conversion
                                        if s.len() == 0 { None } else { Some(s) }
                                    }
                                } else {
                                    quote! {
                                        #string_conversion
                                        s
                                    }
                                }
                            }
                            Type::Fd => quote!(val),
                            Type::Object => {
                                let map_lookup = quote!(map.get(val).ok_or(())?);
                                if arg.allow_null {
                                    quote!(if val == 0 { None } else { Some(#map_lookup) })
                                } else {
                                    map_lookup
                                }
                            }
                            Type::NewId => {
                                let map_lookup = quote!(map.get_new(val).ok_or(())?);
                                if arg.allow_null {
                                    quote!(if val == 0 { None } else { Some(#map_lookup) })
                                } else {
                                    map_lookup
                                }
                            }
                            Type::Destructor => panic!("An argument cannot have type destructor!"),
                        };

                        let common_type = arg.typ.common_type();

                        quote! {
                            #field_name: {
                                if let Some(Argument::#common_type(val)) = args.next() {
                                    #some_code_path
                                } else {
                                    return Err(());
                                }
                            }
                        }
                    });

                    quote! {
                        {
                            let mut args = msg.args.into_iter();

                            Ok(#msg_type_qualified {
                                #(#fields,)*
                            })
                        }
                    }
                };

                quote!(#pattern => #block)
            })
            .chain(iter::once(quote!(_ => Err(()))));

        quote! {
            match msg.opcode {
                #(#match_arms,)*
            }
        }
    } else {
        let panic_message = format!("{}::from_raw can not be used {:?}-side.", name, side);
        quote!(panic!(#panic_message))
    };

    let into_raw_body = if receiver {
        let panic_message = format!("{}::into_raw can not be used {:?}-side.", name, side);
        quote!(panic!(#panic_message))
    } else {
        let match_arms = messages.iter().enumerate().map(|(opcode, msg)| {
            let msg_type = Ident::new(&snake_to_camel(&msg.name), Span::call_site());
            let msg_type_qualified = quote!(#name::#msg_type);

            let pattern = if msg.args.is_empty() {
                msg_type_qualified
            } else {
                let fields = msg
                    .args
                    .iter()
                    .map(|arg| Ident::new(&arg.name, Span::call_site()));
                quote!(#msg_type_qualified { #(#fields),* })
            };

            let opcode_value = Literal::u16_unsuffixed(opcode as u16);
            let args_values = msg.args.iter().map(|arg| {
                let arg_ident = Ident::new(&arg.name, Span::call_site());
                match arg.typ {
                    Type::Int => {
                        if arg.enum_.is_some() {
                            quote!(Argument::Int(#arg_ident.to_raw() as i32))
                        } else {
                            quote!(Argument::Int(#arg_ident))
                        }
                    }
                    Type::Uint => {
                        if arg.enum_.is_some() {
                            quote!(Argument::Uint(#arg_ident.to_raw()))
                        } else {
                            quote!(Argument::Uint(#arg_ident))
                        }
                    }
                    Type::Fixed => quote!(Argument::Fixed((#arg_ident * 256.) as i32)),
                    Type::String => {
                        if arg.allow_null {
                            quote! {
                                Argument::Str(unsafe {
                                    ::std::ffi::CString::from_vec_unchecked(
                                        #arg_ident.map(Into::into).unwrap_or_else(Vec::new),
                                    )
                                })
                            }
                        } else {
                            quote! {
                                Argument::Str(unsafe {
                                    ::std::ffi::CString::from_vec_unchecked(#arg_ident.into())
                                })
                            }
                        }
                    }
                    Type::Array => {
                        if arg.allow_null {
                            quote!(Argument::Array(#arg_ident.unwrap_or_else(Vec::new)))
                        } else {
                            quote!(Argument::Array(#arg_ident))
                        }
                    }
                    Type::Fd => quote!(Argument::Fd(#arg_ident)),
                    Type::NewId => {
                        if arg.interface.is_some() {
                            quote!(Argument::NewId(#arg_ident.id()))
                        } else {
                            quote! {
                                Argument::Str(unsafe {
                                    ::std::ffi::CString::from_vec_unchecked(#arg_ident.0.into())
                                }),
                                Argument::Uint(#arg_ident.1),
                                Argument::NewId(#arg_ident.2.id())
                            }
                        }
                    }
                    Type::Object => {
                        if arg.allow_null {
                            quote!(Argument::Object(#arg_ident.map(|o| o.id()).unwrap_or(0)))
                        } else {
                            quote!(Argument::Object(#arg_ident.id()))
                        }
                    }
                    Type::Destructor => panic!("An argument cannot have type Destructor"),
                }
            });

            quote!(#pattern => Message {
                sender_id: sender_id,
                opcode: #opcode_value,
                args: vec![
                    #(#args_values,)*
                ],
            })
        });

        quote! {
            match self {
                #(#match_arms,)*
            }
        }
    };

    quote! {
        pub enum #name {
            #(#variants,)*
        }

        impl super::MessageGroup for #name {
            const MESSAGES: &'static [super::MessageDesc] = &[
                #(#message_array_values,)*
            ];

            type Map = super::#map_type;

            fn is_destructor(&self) -> bool {
                match *self {
                    #(#is_destructor_match_arms,)*
                }
            }

            fn opcode(&self) -> u16 {
                match *self {
                    #(#opcode_match_arms,)*
                }
            }

            fn child<Meta: ObjectMetadata>(opcode: u16, version: u32, meta: &Meta) -> Option<Object<Meta>> {
                match opcode {
                    #(#child_match_arms,)*
                }
            }

            fn from_raw(msg: Message, map: &mut Self::Map) -> Result<Self, ()> {
                #from_raw_body
            }

            fn into_raw(self, sender_id: u32) -> Message {
                #into_raw_body
            }

            #addon
        }
    }
}

pub(crate) fn gen_interface(
    name: &Ident,
    low_name: &str,
    version: u32,
    addon: Option<TokenStream>,
) -> TokenStream {
    let version_lit = Literal::u32_unsuffixed(version);

    quote! {
        pub struct #name;

        impl Interface for #name {
            type Request = Request;
            type Event = Event;
            const NAME: &'static str = #low_name;
            const VERSION: u32 = #version_lit;

            #addon
        }
    }
}

pub fn method_prototype<'a>(iname: &Ident, msg: &'a Message) -> (TokenStream, Option<&'a Arg>) {
    let mut it = msg.args.iter().filter(|arg| arg.typ == Type::NewId);
    let newid = it.next();
    assert!(
        newid.is_none() || it.next().is_none(),
        "Request {}.{} returns more than one new_id",
        iname,
        msg.name
    );

    let fn_name = Ident::new(
        &format!("{}{}", if is_keyword(&msg.name) { "_" } else { "" }, msg.name),
        Span::call_site(),
    );

    let mut args = Vec::new();

    let generics = if let Some(arg) = newid {
        if arg.interface.is_none() {
            args.push(quote!(version: u32));
            Some(quote!(T: Interface, F))
        } else {
            Some(quote!(F))
        }
    } else {
        None
    };

    args.extend(msg.args.iter().filter_map(|arg| {
        let arg_type_inner = if let Some(ref name) = arg.enum_ {
            dotted_to_relname(name)
        } else {
            match arg.typ {
                Type::Object => arg
                    .interface
                    .as_ref()
                    .map(|iface| {
                        let iface_mod = Ident::new(iface, Span::call_site());
                        let iface_type = Ident::new(&snake_to_camel(iface), Span::call_site());
                        quote!(&Proxy<super::#iface_mod::#iface_type>)
                    })
                    .unwrap_or(quote!(&Proxy<super::AnonymousObject>)),
                Type::NewId => {
                    // client-side, the return-type handles that
                    return None;
                }
                _ => arg.typ.rust_type(),
            }
        };

        let arg_name = Ident::new(
            &format!("{}{}", if is_keyword(&arg.name) { "_" } else { "" }, arg.name),
            Span::call_site(),
        );

        let arg_type = if arg.allow_null {
            quote!(Option<#arg_type_inner>)
        } else {
            arg_type_inner
        };

        Some(quote!(#arg_name: #arg_type))
    }));

    if newid.is_some() {
        args.push(quote!(implementor: F));
    }

    let (return_type, where_bounds) = if let Some(arg) = newid {
        match arg.interface {
            Some(ref iface) => {
                let iface_mod = Ident::new(&iface, Span::call_site());
                let iface_type = Ident::new(&snake_to_camel(&iface), Span::call_site());

                (
                    quote!(Result<Proxy<super::#iface_mod::#iface_type>, ()>),
                    Some(quote! {
                        where F: FnOnce(
                            NewProxy<super::#iface_mod::#iface_type>,
                        ) -> Proxy<super::#iface_mod::#iface_type>
                    }),
                )
            }
            None => (
                quote!(Result<Proxy<T>, ()>),
                Some(quote!(where F: FnOnce(NewProxy<T>) -> Proxy<T>)),
            ),
        }
    } else {
        (quote!(()), None)
    };

    let prototype = quote! {
        fn #fn_name#(<#generics>)*(&self, #(#args),*) -> #return_type #where_bounds
    };

    (prototype, newid)
}

pub(crate) fn gen_client_methods(name: &Ident, messages: &[Message]) -> TokenStream {
    let methods = messages.iter().map(|msg| {
        let mut docs = String::new();
        if let Some((ref short, ref long)) = msg.description {
            docs += &format!("{}\n\n{}\n", short, long);
        }
        if let Some(Type::Destructor) = msg.typ {
            docs += "\nThis is a destructor, you cannot send requests to this object any longer once this method is called.";
        }
        if msg.since > 1 {
            docs += &format!("\nOnly available since version {} of the interface.", msg.since);
        }

        let doc_attr = to_doc_attr(&docs);
        let (proto, _) = method_prototype(name, &msg);

        quote! {
            #doc_attr
            #proto;
        }
    });

    let method_impls = messages.iter().map(|msg| {
        let msg_name = Ident::new(&snake_to_camel(&msg.name), Span::call_site());
        let (proto, return_type) = method_prototype(name, &msg);

        let msg_init = if msg.args.is_empty() {
            TokenStream::new()
        } else {
            let args = msg.args.iter().map(|arg| {
                let arg_name = Ident::new(&arg.name, Span::call_site());
                let arg_value = match arg.typ {
                    Type::NewId => {
                        if arg.interface.is_some() {
                            quote!(self.child_placeholder())
                        } else {
                            quote!((T::NAME.into(), version, self.child_placeholder()))
                        }
                    }
                    Type::Object => {
                        if arg.allow_null {
                            quote!(#arg_name.map(|o| o.clone()))
                        } else {
                            quote!(#arg_name.clone())
                        }
                    }
                    _ => quote!(#arg_name),
                };

                quote!(#arg_name: #arg_value)
            });

            quote!({ #(#args),* })
        };

        let send_stmt = match return_type {
            Some(ret_type) if ret_type.interface.is_none() => {
                quote!(self.send_constructor(msg, implementor, Some(version)))
            }
            Some(_) => quote!(self.send_constructor(msg, implementor, None)),
            None => quote! {
                self.send(msg);
            },
        };

        quote! {
            #proto {
                let msg = Request::#msg_name #msg_init;
                #send_stmt
            }
        }
    });

    quote! {
        pub trait RequestsTrait {
            #(#methods)*
        }

        impl RequestsTrait for Proxy<#name> {
            #(#method_impls)*
        }
    }
}
