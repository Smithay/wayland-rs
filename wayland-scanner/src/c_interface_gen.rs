use std::cmp;
use std::iter::repeat;

use proc_macro2::{Ident, Literal, Span, TokenStream};
use quote::quote;

use crate::protocol::*;
use crate::util::null_terminated_byte_string_literal;

pub(crate) fn generate_interfaces_prefix(protocol: &Protocol) -> TokenStream {
    let longest_nulls = protocol.interfaces.iter().fold(0, |max, interface| {
        let request_longest_null = interface.requests.iter().fold(0, |max, request| {
            if request.all_null() {
                cmp::max(request.args.len(), max)
            } else {
                max
            }
        });
        let events_longest_null = interface.events.iter().fold(0, |max, event| {
            if event.all_null() {
                cmp::max(event.args.len(), max)
            } else {
                max
            }
        });
        cmp::max(max, cmp::max(request_longest_null, events_longest_null))
    });

    let types_null_len = Literal::usize_unsuffixed(longest_nulls);

    let nulls = repeat(quote!(NULLPTR as *const sys::common::wl_interface)).take(longest_nulls);

    quote! {
        use std::os::raw::{c_char, c_void};

        const NULLPTR: *const c_void = 0 as *const c_void;
        static mut types_null: [*const sys::common::wl_interface; #types_null_len] = [
            #(#nulls,)*
        ];
    }
}

pub(crate) fn generate_interface(interface: &Interface) -> TokenStream {
    let requests = gen_messages(interface, &interface.requests, "requests");
    let events = gen_messages(interface, &interface.events, "events");

    let interface_ident = Ident::new(&format!("{}_interface", interface.name), Span::call_site());
    let name_value = null_terminated_byte_string_literal(&interface.name);
    let version_value = Literal::i32_unsuffixed(interface.version as i32);
    let request_count_value = Literal::i32_unsuffixed(interface.requests.len() as i32);
    let requests_value = if interface.requests.is_empty() {
        quote!(NULLPTR as *const wl_message)
    } else {
        let requests_ident = Ident::new(&format!("{}_requests", interface.name), Span::call_site());
        quote!(unsafe { &#requests_ident as *const _ })
    };
    let event_count_value = Literal::i32_unsuffixed(interface.events.len() as i32);
    let events_value = if interface.events.is_empty() {
        quote!(NULLPTR as *const wl_message)
    } else {
        let events_ident = Ident::new(&format!("{}_events", interface.name), Span::call_site());
        quote!(unsafe { &#events_ident as *const _ })
    };

    quote!(
        #requests
        #events

        /// C representation of this interface, for interop
        pub static mut #interface_ident: wl_interface = wl_interface {
            name: #name_value as *const u8 as *const c_char,
            version: #version_value,
            request_count: #request_count_value,
            requests: #requests_value,
            event_count: #event_count_value,
            events: #events_value,
        };
    )
}

fn gen_messages(interface: &Interface, messages: &[Message], which: &str) -> TokenStream {
    if messages.is_empty() {
        return TokenStream::new();
    }

    let types_arrays = messages.iter().filter_map(|msg| {
        if msg.all_null() {
            None
        } else {
            let array_ident = Ident::new(
                &format!("{}_{}_{}_types", interface.name, which, msg.name),
                Span::call_site(),
            );
            let array_len = Literal::usize_unsuffixed(msg.args.len());
            let array_values = msg.args.iter().map(|arg| match (arg.typ, &arg.interface) {
                (Type::Object, &Some(ref inter)) | (Type::NewId, &Some(ref inter)) => {
                    let module = Ident::new(inter, Span::call_site());
                    let interface_ident =
                        Ident::new(&format!("{}_interface", inter), Span::call_site());
                    quote!(unsafe { &super::#module::#interface_ident as *const wl_interface })
                }
                _ => quote!(NULLPTR as *const wl_interface),
            });

            Some(quote! {
                static mut #array_ident: [*const wl_interface; #array_len] = [
                    #(#array_values,)*
                ];
            })
        }
    });

    let message_array_ident =
        Ident::new(&format!("{}_{}", interface.name, which), Span::call_site());
    let message_array_len = Literal::usize_unsuffixed(messages.len());
    let message_array_values = messages.iter().map(|msg| {
        let name_value = null_terminated_byte_string_literal(&msg.name);
        let signature_value = Literal::byte_string(&message_signature(msg));

        let types_ident = if msg.all_null() {
            Ident::new("types_null", Span::call_site())
        } else {
            Ident::new(
                &format!("{}_{}_{}_types", interface.name, which, msg.name),
                Span::call_site(),
            )
        };

        quote! {
            wl_message {
                name: #name_value as *const u8 as *const c_char,
                signature: #signature_value as *const u8 as *const c_char,
                types: unsafe { &#types_ident as *const _ },
            }
        }
    });

    quote! {
        #(#types_arrays)*

        /// C-representation of the messages of this interface, for interop
        pub static mut #message_array_ident: [wl_message; #message_array_len] = [
            #(#message_array_values,)*
        ];
    }
}

fn message_signature(msg: &Message) -> Vec<u8> {
    let mut res = Vec::new();

    if msg.since > 1 {
        res.extend_from_slice(msg.since.to_string().as_bytes());
    }

    for arg in &msg.args {
        if arg.typ.nullable() && arg.allow_null {
            res.push(b'?');
        }
        match arg.typ {
            Type::NewId => {
                if arg.interface.is_none() {
                    res.extend_from_slice(b"su");
                }
                res.push(b'n');
            }
            Type::Uint => res.push(b'u'),
            Type::Fixed => res.push(b'f'),
            Type::String => res.push(b's'),
            Type::Object => res.push(b'o'),
            Type::Array => res.push(b'a'),
            Type::Fd => res.push(b'h'),
            Type::Int => res.push(b'i'),
            _ => {}
        }
    }

    res.push(0);
    res
}
