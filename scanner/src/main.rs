extern crate xml;

mod parse;
mod protocol;

fn main() {
    let protocol = parse::parse_stream(std::io::stdin());
}
