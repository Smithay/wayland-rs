use std::iter;

use proc_macro2::{Ident, Literal, Span, TokenStream};
use quote::quote;

use crate::common_gen::*;
use crate::protocol::*;
use crate::util::*;
use crate::Side;

pub(crate) fn generate_protocol_client(protocol: Protocol) -> TokenStream {
    // Force the fallback to work around https://github.com/alexcrichton/proc-macro2/issues/218
    proc_macro2::fallback::force();

    let modules = protocol.interfaces.iter().map(|iface| {
        let doc_attr = iface.description.as_ref().map(description_to_doc_attr);
        let mod_name = Ident::new(&iface.name, Span::call_site());
        let iface_name = Ident::new(&snake_to_camel(&iface.name), Span::call_site());

        let enums = &iface.enums;

        let ident = Ident::new("Request", Span::call_site());
        let requests = gen_messagegroup(
            &ident,
            Side::Client,
            false,
            &iface.requests,
            Some(messagegroup_c_addon(&ident, &iface_name, Side::Client, false, &iface.requests)),
        );

        let ident = Ident::new("Event", Span::call_site());
        let events = gen_messagegroup(
            &ident,
            Side::Client,
            true,
            &iface.events,
            Some(messagegroup_c_addon(&ident, &iface_name, Side::Client, true, &iface.events)),
        );

        let interface = gen_interface(
            &iface_name,
            &iface.name,
            iface.version,
            Some(interface_c_addon(&iface.name)),
            Side::Client,
        );

        let object_methods = gen_object_methods(&iface_name, &iface.requests, Side::Client);
        let sinces = gen_since_constants(&iface.requests, &iface.events);
        let c_interface = super::c_interface_gen::generate_interface(iface);

        quote! {
            #doc_attr
            pub mod #mod_name {
                use std::os::raw::c_char;
                use super::{
                    Proxy, AnonymousObject, Interface, MessageGroup, MessageDesc, ArgumentType,
                    Object, Message, Argument, ObjectMetadata, types_null, NULLPTR, Main, smallvec,
                };
                use super::sys::common::{wl_interface, wl_array, wl_argument, wl_message};
                use super::sys::client::*;

                #(#enums)*
                #requests
                #events
                #interface
                #object_methods
                #sinces
                #c_interface
            }
        }
    });

    let c_prefix = super::c_interface_gen::generate_interfaces_prefix(&protocol);

    quote! {
        #c_prefix

        #(#modules)*
    }
}

pub(crate) fn generate_protocol_server(protocol: Protocol) -> TokenStream {
    // Force the fallback to work around https://github.com/alexcrichton/proc-macro2/issues/218
    proc_macro2::fallback::force();

    let modules = protocol
        .interfaces
        .iter()
        // display and registry are handled specially
        .filter(|iface| iface.name != "wl_display" && iface.name != "wl_registry")
        .map(|iface| {
            let doc_attr = iface.description.as_ref().map(description_to_doc_attr);
            let mod_name = Ident::new(&iface.name, Span::call_site());
            let iface_name = Ident::new(&snake_to_camel(&iface.name), Span::call_site());

            let enums = &iface.enums;

            let ident = Ident::new("Request", Span::call_site());
            let requests = gen_messagegroup(
                &ident,
                Side::Server,
                true,
                &iface.requests,
                Some(messagegroup_c_addon(
                    &ident,
                    &iface_name,
                    Side::Server,
                    true,
                    &iface.requests,
                )),
            );

            let ident = Ident::new("Event", Span::call_site());
            let events = gen_messagegroup(
                &ident,
                Side::Server,
                false,
                &iface.events,
                Some(messagegroup_c_addon(
                    &ident,
                    &iface_name,
                    Side::Server,
                    false,
                    &iface.events,
                )),
            );

            let interface = gen_interface(
                &Ident::new(&snake_to_camel(&iface.name), Span::call_site()),
                &iface.name,
                iface.version,
                Some(interface_c_addon(&iface.name)),
                Side::Server,
            );
            let object_methods = gen_object_methods(&iface_name, &iface.events, Side::Server);
            let sinces = gen_since_constants(&iface.requests, &iface.events);
            let c_interface = super::c_interface_gen::generate_interface(iface);

            quote! {
                #doc_attr
                pub mod #mod_name {
                    use std::os::raw::c_char;
                    use super::{
                        Resource, AnonymousObject, Interface, MessageGroup, MessageDesc, Main, smallvec,
                        ArgumentType, Object, Message, Argument, ObjectMetadata, types_null, NULLPTR
                    };
                    use super::sys::common::{wl_argument, wl_interface, wl_array, wl_message};
                    use super::sys::server::*;

                    #(#enums)*
                    #requests
                    #events
                    #interface
                    #object_methods
                    #sinces
                    #c_interface
                }
            }
        });

    let c_prefix = super::c_interface_gen::generate_interfaces_prefix(&protocol);

    quote! {
        #c_prefix
        #(#modules)*
    }
}

fn messagegroup_c_addon(
    name: &Ident,
    parent_iface: &Ident,
    side: Side,
    receiver: bool,
    messages: &[Message],
) -> TokenStream {
    let from_raw_c_body = if receiver {
        let match_arms = messages
            .iter()
            .enumerate()
            .map(|(i, msg)| {
                let pattern = Literal::u16_unsuffixed(i as u16);
                let msg_name = Ident::new(&snake_to_camel(&msg.name), Span::call_site());
                let msg_name_qualified = quote!(#name::#msg_name);
                let (args_binding, result) = if msg.args.is_empty() {
                    (None, msg_name_qualified)
                } else {
                    let len = Literal::usize_unsuffixed(msg.args.len());

                    let fields = msg.args.iter().enumerate().map(|(j, arg)| {
                let field_name = Ident::new(
                    &format!("{}{}", if is_keyword(&arg.name) { "_" } else { "" }, arg.name),
                    Span::call_site(),
                );

                        let idx = Literal::usize_unsuffixed(j);
                        let field_value = match arg.typ {
                            Type::Uint => {
                                if let Some(ref enu) = arg.enum_ {
                                    let enum_type = dotted_to_relname(enu);
                                    quote!(#enum_type::from_raw(_args[#idx].u).ok_or(())?)
                                } else {
                                    quote!(_args[#idx].u)
                                }
                            }
                            Type::Int => {
                                if let Some(ref enu) = arg.enum_ {
                                    let enum_type = dotted_to_relname(enu);
                                    quote!(#enum_type::from_raw(_args[#idx].i as u32).ok_or(())?)
                                } else {
                                    quote!(_args[#idx].i)
                                }
                            }
                            Type::Fixed => quote!((_args[#idx].f as f64) / 256.),
                            Type::String => {
                                let string_conversion = quote! {
                                    ::std::ffi::CStr::from_ptr(_args[#idx].s).to_string_lossy().into_owned()
                                };

                                if arg.allow_null {
                                    quote! {
                                        if _args[#idx].s.is_null() { None } else { Some(#string_conversion) }
                                    }
                                } else {
                                    string_conversion
                                }
                            }
                            Type::Array => {
                                let array_conversion = quote! {
                                    {
                                        let array = &*_args[#idx].a;
                                        ::std::slice::from_raw_parts(array.data as *const u8, array.size)
                                            .to_owned()
                                    }
                                };

                                if arg.allow_null {
                                    quote! {
                                        if _args[#idx].a.is_null() { None } else { Some(#array_conversion) }
                                    }
                                } else {
                                    array_conversion
                                }
                            }
                            Type::Fd => quote!(_args[#idx].h),
                            Type::Object => {
                                let object_name = side.object_name();
                                let object_conversion = if let Some(ref iface) = arg.interface {
                                    let iface_mod = Ident::new(iface, Span::call_site());
                                    let iface_type = Ident::new(&snake_to_camel(iface), Span::call_site());

                                    quote! {
                                        #object_name::<super::#iface_mod::#iface_type>::from_c_ptr(
                                            _args[#idx].o as *mut _,
                                        ).into()
                                    }
                                } else {
                                    quote! {
                                        #object_name::<AnonymousObject>::from_c_ptr(_args[#idx].o as *mut _).into()
                                    }
                                };

                                if arg.allow_null {
                                    quote! {
                                        if _args[#idx].o.is_null() { None } else { Some(#object_conversion) }
                                    }
                                } else {
                                    object_conversion
                                }
                            }
                            Type::NewId => {
                                let new_id_conversion = if let Some(ref iface) = arg.interface {
                                    let iface_mod = Ident::new(iface, Span::call_site());
                                    let iface_type = Ident::new(&snake_to_camel(iface), Span::call_site());

                                    match side {
                                        Side::Client => {
                                            quote! {
                                                Main::<super::#iface_mod::#iface_type>::from_c_ptr(
                                                    _args[#idx].o as *mut _
                                                )
                                            }
                                        }
                                        Side::Server => {
                                            quote! {
                                                {
                                                    let me = Resource::<#parent_iface>::from_c_ptr(obj as *mut _);
                                                    me.make_child_for::<super::#iface_mod::#iface_type>(_args[#idx].n).unwrap()
                                                }
                                            }
                                        }
                                    }
                                } else {
                                    // bind-like function
                                    quote!(panic!("Cannot unserialize anonymous new id."))
                                };

                                if arg.allow_null {
                                    quote! {
                                        if _args[#idx].o.is_null() { None } else { Some(#new_id_conversion) }
                                    }
                                } else {
                                    new_id_conversion
                                }
                            }
                            Type::Destructor => panic!("An argument cannot have type \"destructor\"."),
                        };

                        quote!(#field_name: #field_value)
                    });

                    let result = quote! {
                        #msg_name_qualified {
                            #(#fields,)*
                        }
                    };

                    let args_binding = quote! {
                        let _args = ::std::slice::from_raw_parts(args, #len);
                    };

                    (Some(args_binding), result)
                };

                quote! {
                    #pattern => {
                        #args_binding
                        Ok(#result)
                    }
                }
            })
            .chain(iter::once(quote!(_ => return Err(()))));

        quote! {
            match opcode {
                #(#match_arms,)*
            }
        }
    } else {
        let panic_message = format!("{}::from_raw_c can not be used {:?}-side.", name, side);
        quote!(panic!(#panic_message))
    };

    let as_raw_c_in_body = if receiver {
        let panic_message = format!("{}::as_raw_c_in can not be used {:?}-side.", name, side);
        quote!(panic!(#panic_message))
    } else {
        let match_arms = messages.iter().enumerate().map(|(i, msg)| {
            let msg_name = Ident::new(&snake_to_camel(&msg.name), Span::call_site());
            let pattern = if msg.args.is_empty() {
                quote!(#name::#msg_name)
            } else {
                let fields = msg.args.iter().flat_map(|arg| {
                    // Client-side newid request do not contain a placeholder
                    if side == Side::Client && arg.typ == Type::NewId && arg.interface.is_some() {
                        None
                    } else {
                        Some(Ident::new(
                            &format!("{}{}", if is_keyword(&arg.name) { "_" } else { "" }, arg.name),
                            Span::call_site(),
                        ))
                    }
                });

                quote!(#name::#msg_name { #(#fields),* })
            };

            let buffer_len = Literal::usize_unsuffixed(
                msg.args.len()
                    + 2 * msg
                        .args
                        .iter()
                        .filter(|arg| arg.typ == Type::NewId && arg.interface.is_none())
                        .count(),
            );

            let mut j = 0;
            let args_array_init_stmts = msg.args.iter().map(|arg| {
                let idx = Literal::usize_unsuffixed(j);
                let arg_name = Ident::new(
                    &format!("{}{}", if is_keyword(&arg.name) { "_" } else { "" }, arg.name),
                    Span::call_site(),
                );

                let res = match arg.typ {
                    Type::Uint => {
                        if arg.enum_.is_some() {
                            quote! {
                                _args_array[#idx].u = #arg_name.to_raw();
                            }
                        } else {
                            quote! {
                                _args_array[#idx].u = #arg_name;
                            }
                        }
                    }
                    Type::Int => {
                        if arg.enum_.is_some() {
                            quote! {
                                _args_array[#idx].i = #arg_name.to_raw() as i32;
                            }
                        } else {
                            quote! {
                                _args_array[#idx].i = #arg_name;
                            }
                        }
                    }
                    Type::Fixed => quote! {
                        _args_array[#idx].f = (#arg_name * 256.) as i32;
                    },
                    Type::String => {
                        let arg_variable = Ident::new(&format!("_arg_{}", j), Span::call_site());
                        if arg.allow_null {
                            quote! {
                                let #arg_variable = #arg_name.map(|s| ::std::ffi::CString::new(s).unwrap());
                                _args_array[#idx].s =
                                    (&#arg_variable).as_ref().map(|s| s.as_ptr()).unwrap_or(::std::ptr::null());
                            }
                        } else {
                            quote! {
                                let #arg_variable = ::std::ffi::CString::new(#arg_name).unwrap();
                                _args_array[#idx].s = #arg_variable.as_ptr();
                            }
                        }
                    }
                    Type::Array => {
                        let arg_variable = Ident::new(&format!("_arg_{}", j), Span::call_site());
                        if arg.allow_null {
                            quote! {
                                let #arg_variable = #arg_name.as_ref().map(|vec| wl_array {
                                    size: vec.len(),
                                    alloc: vec.capacity(),
                                    data: vec.as_ptr() as *mut _,
                                });
                                _args_array[#idx].a = #arg_variable
                                    .as_ref()
                                    .map(|a| a as *const wl_array)
                                    .unwrap_or(::std::ptr::null());
                            }
                        } else {
                            quote! {
                                let #arg_variable = wl_array {
                                    size: #arg_name.len(),
                                    alloc: #arg_name.capacity(),
                                    data: #arg_name.as_ptr() as *mut _,
                                };
                                _args_array[#idx].a = &#arg_variable;
                            }
                        }
                    }
                    Type::Fd => quote! {
                        _args_array[#idx].h = #arg_name;
                    },
                    Type::Object => {
                        if arg.allow_null {
                            quote! {
                                _args_array[#idx].o = #arg_name
                                    .map(|o| o.as_ref().c_ptr() as *mut _)
                                    .unwrap_or(::std::ptr::null_mut());
                            }
                        } else {
                            quote! {
                                _args_array[#idx].o = #arg_name.as_ref().c_ptr() as *mut _;
                            }
                        }
                    }
                    Type::NewId => {
                        if arg.interface.is_some() {
                            if side == Side::Client {
                                quote! {
                                    _args_array[#idx].o = ::std::ptr::null_mut() as *mut _;
                                }
                            } else {
                                quote! {
                                    _args_array[#idx].o = #arg_name.c_ptr() as *mut _;
                                }
                            }
                        } else {
                            assert!(
                                side != Side::Server,
                                "Cannot serialize anonymous NewID from server."
                            );

                            // The arg is actually (string, uint, NULL)
                            let arg_variable = Ident::new(&format!("_arg_{}_s", j), Span::call_site());
                            let idx1 = Literal::usize_unsuffixed(j + 1);
                            let idx2 = Literal::usize_unsuffixed(j + 2);

                            let res = quote! {
                                let #arg_variable = ::std::ffi::CString::new(#arg_name.0).unwrap();
                                _args_array[#idx].s = #arg_variable.as_ptr();
                                _args_array[#idx1].u = #arg_name.1;
                                _args_array[#idx2].o = ::std::ptr::null_mut();
                            };

                            j += 2;

                            res
                        }
                    }
                    Type::Destructor => panic!("An argument cannot have type \"destructor\"."),
                };

                j += 1;

                res
            });

            let idx = Literal::u32_unsuffixed(i as u32);

            quote! {
                #pattern => {
                    let mut _args_array: [wl_argument; #buffer_len] = unsafe { ::std::mem::zeroed() };
                    #(#args_array_init_stmts)*

                    f(#idx, &mut _args_array)
                }
            }
        });

        quote! {
            match self {
                #(#match_arms,)*
            }
        }
    };

    quote! {
        unsafe fn from_raw_c(
            obj: *mut ::std::os::raw::c_void,
            opcode: u32,
            args: *const wl_argument,
        ) -> Result<#name, ()> {
            #from_raw_c_body
        }

        fn as_raw_c_in<F, T>(self, f: F) -> T where F: FnOnce(u32, &mut [wl_argument]) -> T {
            #as_raw_c_in_body
        }
    }
}

fn interface_c_addon(low_name: &str) -> TokenStream {
    let iface_name = Ident::new(&format!("{}_interface", low_name), Span::call_site());
    quote! {
        fn c_interface() -> *const wl_interface {
            unsafe { &#iface_name }
        }
    }
}
