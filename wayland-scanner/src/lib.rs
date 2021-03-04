use std::{ffi::OsString, path::PathBuf};

use wayland_commons::scanner;

use syn::{parse_macro_input, LitStr};

mod interfaces;

#[proc_macro]
pub fn generate_interfaces(stream: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let path: OsString = parse_macro_input!(stream as LitStr).value().into();
    let path = if let Some(manifest_dir) = std::env::var_os("CARGO_MANIFEST_DIR") {
        let mut buf = PathBuf::from(manifest_dir);
        buf.push(path);
        buf
    } else {
        path.into()
    };
    let file = match std::fs::File::open(&path) {
        Ok(file) => file,
        Err(e) => panic!("Failed to open protocol file {}: {}", path.display(), e),
    };
    let protocol = scanner::parse(file);
    interfaces::generate(&protocol).into()
}

#[cfg(test)]
fn format_rust_code(code: &str) -> String {
    use std::{
        io::Write,
        process::{Command, Stdio},
    };
    if let Ok(mut proc) = Command::new("rustfmt")
        .arg("--emit=stdout")
        .arg("--edition=2018")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        //.stderr(Stdio::null())
        .spawn()
    {
        {
            let stdin = proc.stdin.as_mut().unwrap();
            stdin.write_all(code.as_bytes()).unwrap();
        }
        if let Ok(output) = proc.wait_with_output() {
            if output.status.success() {
                return std::str::from_utf8(&output.stdout).unwrap().to_owned().into();
            }
        }
    }
    panic!("Rustfmt failed!");
}
