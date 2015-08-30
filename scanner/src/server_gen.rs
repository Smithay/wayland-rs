use protocol::*;

pub fn generate_server_api(protocol: Protocol) {
    if let Some(text) = protocol.copyright {
        println!("/*\n{}\n*/\n", text);
    }
    
}