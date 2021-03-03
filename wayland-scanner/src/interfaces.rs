use proc_macro2::{Ident, Span, TokenStream};

use wayland_commons::scanner::{Interface, Message, Protocol, Type};

use quote::quote;

pub fn generate(protocol: &Protocol) -> TokenStream {
    let interfaces = protocol.interfaces.iter().map(generate_interface);
    quote!( #(#interfaces)* )
}

fn generate_interface(interface: &Interface) -> TokenStream {
    let const_name = Ident::new(
        &format!("{}_INTERFACE", interface.name.to_ascii_uppercase()),
        Span::call_site(),
    );
    let iface_name = &interface.name;
    let iface_version = interface.version;
    let requests = build_messagedesc_list(&interface.requests, &interface.name);
    let events = build_messagedesc_list(&interface.events, &interface.name);

    quote!(
        pub static #const_name: wayland_commons::Interface = wayland_commons::Interface {
            name: #iface_name,
            version: #iface_version,
            requests: #requests,
            events: #events,
        };
    )
}

fn build_messagedesc_list(list: &[Message], iface: &str) -> TokenStream {
    let desc_list = list.iter().map(|message| {
        let name = &message.name;
        let since = message.since;
        let is_destructor = message.typ == Some(Type::Destructor);
        let signature = message.args.iter().map(|arg| arg.typ.common_type());
        let child_interface = match message
            .args
            .iter()
            .find(|arg| arg.typ == Type::NewId)
            .and_then(|arg| arg.interface.as_ref())
        {
            Some(name) => {
                let target_iface = Ident::new(
                    &format!("{}_INTERFACE", name.to_ascii_uppercase()),
                    Span::call_site(),
                );
                quote!( Some(&#target_iface) )
            }
            None => quote!(None),
        };
        let arg_interfaces = message.args.iter().filter(|arg| arg.typ == Type::Object).map(|arg| {
            match arg.interface {
                Some(ref name) => {
                    let target_iface = Ident::new(
                        &format!("{}_INTERFACE", name.to_ascii_uppercase()),
                        Span::call_site(),
                    );
                    quote!( &#target_iface )
                }
                None => {
                    panic!("Unsupported anonymous object argument in {}.{}", iface, message.name);
                }
            }
        });
        quote!(
            wayland_commons::MessageDesc {
                name: #name,
                signature: &[ #(wayland_commons::ArgumentType::#signature),* ],
                since: #since,
                is_destructor: #is_destructor,
                child_interface: #child_interface,
                arg_interfaces: &[ #(#arg_interfaces),* ],
            }
        )
    });

    quote!(
        &[ #(#desc_list)* ]
    )
}

#[cfg(test)]
mod tests {
    use proc_macro2::TokenStream;

    #[test]
    fn interface_gen() {
        let protocol_file =
            std::fs::File::open("../tests/scanner_assets/test-protocol.xml").unwrap();
        let protocol_parsed = wayland_commons::scanner::parse(protocol_file);
        let generated: String = super::generate(&protocol_parsed).to_string();
        let generated = crate::format_rust_code(&generated);

        let reference =
            std::fs::read_to_string("../tests/scanner_assets/test-interfaces.rs").unwrap();
        let reference = crate::format_rust_code(&reference);

        if reference != generated {
            let diff = similar::TextDiff::from_lines(&reference, &generated);
            print!("{}", diff.unified_diff().context_radius(10).header("reference", "generated"));
            panic!("Generated does not match reference!")
        }
    }
}
