use protocol::*;

pub fn generate_client_api(protocol: Protocol) {
    if let Some(text) = protocol.copyright {
        println!("/*\n{}\n*/\n", text);
    }
    
}