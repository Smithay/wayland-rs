use std::io::Result as IOResult;
use std::cmp;
use std::io::Write;

use protocol::*;
use util::*;

pub(crate) fn write_prefix<O: Write>(protocol: &Protocol, out: &mut O) -> IOResult<()> {
    writeln!(
        out,
        r#"
//
// This file was auto-generated, do not edit directly.
//
"#
    )?;
    if let Some(ref text) = protocol.copyright {
        writeln!(out, "/*\n{}\n*/\n", text)?;
    }
    Ok(())
}

pub(crate) fn write_enums<O: Write>(enums: &[Enum], out: &mut O) -> IOResult<()> {
    // generate contents
    for enu in enums {
        if enu.bitfield {
            writeln!(out, "    bitflags! {{")?;
            if let Some((ref short, ref long)) = enu.description {
                write_doc(Some(short), long, false, out, 2)?;
            }
            writeln!(
                out,
                "        pub struct {}: u32 {{",
                snake_to_camel(&enu.name)
            )?;
            for entry in &enu.entries {
                if let Some((ref short, ref long)) = entry.description {
                    write_doc(Some(short), long, false, out, 3)?;
                } else if let Some(ref summary) = entry.summary {
                    writeln!(out, "            /// {}", summary)?;
                }
                writeln!(
                    out,
                    "            const {}{} = {};",
                    if entry.name.chars().next().unwrap().is_numeric() {
                        "_"
                    } else {
                        ""
                    },
                    snake_to_camel(&entry.name),
                    entry.value
                )?;
            }
            writeln!(out, "        }}")?;
            writeln!(out, "    }}")?;
            writeln!(out, "    impl {} {{", snake_to_camel(&enu.name))?;
            writeln!(
                out,
                "        pub fn from_raw(n: u32) -> Option<{}> {{",
                snake_to_camel(&enu.name)
            )?;
            writeln!(
                out,
                "            Some({}::from_bits_truncate(n))",
                snake_to_camel(&enu.name)
            )?;
            writeln!(
                out,
                r#"
        }}
        pub fn to_raw(&self) -> u32 {{
            self.bits()
        }}
    }}
"#
            )?;
        } else {
            // if enu.bitfield
            if let Some((ref short, ref long)) = enu.description {
                write_doc(Some(short), long, false, out, 1)?;
            }
            writeln!(
                out,
                r#"
    #[repr(u32)]
    #[derive(Copy,Clone,Debug,PartialEq)]
    pub enum {} {{"#,
                snake_to_camel(&enu.name)
            )?;
            for entry in &enu.entries {
                if let Some((ref short, ref long)) = entry.description {
                    write_doc(Some(short), long, false, out, 2)?;
                } else if let Some(ref summary) = entry.summary {
                    writeln!(out, "        /// {}", summary)?;
                }
                writeln!(
                    out,
                    "        {}{} = {},",
                    if entry.name.chars().next().unwrap().is_numeric() {
                        "_"
                    } else {
                        ""
                    },
                    snake_to_camel(&entry.name),
                    entry.value
                )?;
            }
            writeln!(out, "    }}")?;

            writeln!(out, "    impl {} {{", snake_to_camel(&enu.name))?;
            writeln!(
                out,
                "        pub fn from_raw(n: u32) -> Option<{}> {{",
                snake_to_camel(&enu.name)
            )?;
            writeln!(out, "            match n {{")?;
            for entry in &enu.entries {
                writeln!(
                    out,
                    "                {} => Some({}::{}{}),",
                    entry.value,
                    snake_to_camel(&enu.name),
                    if entry.name.chars().next().unwrap().is_numeric() {
                        "_"
                    } else {
                        ""
                    },
                    snake_to_camel(&entry.name)
                )?;
            }
            writeln!(
                out,
                r#"
                _ => Option::None
            }}
        }}
        pub fn to_raw(&self) -> u32 {{
            *self as u32
        }}
    }}
"#
            )?;
        }
    }
    Ok(())
}

pub(crate) fn write_doc<O: Write>(
    short: Option<&str>,
    long: &str,
    internal: bool,
    out: &mut O,
    indent: usize,
) -> IOResult<()> {
    let p = if internal { '!' } else { '/' };
    if let Some(txt) = short {
        for _ in 0..indent {
            write!(out, "    ")?
        }
        writeln!(out, "//{} {}", p, txt)?;
        for _ in 0..indent {
            write!(out, "    ")?
        }
        writeln!(out, "//{}", p)?;
    }
    for l in long.lines() {
        for _ in 0..indent {
            write!(out, "    ")?
        }
        writeln!(out, "//{} {}", p, l.trim())?;
    }
    Ok(())
}
