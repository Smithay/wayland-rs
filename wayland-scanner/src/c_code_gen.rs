use std::io::Result as IOResult;
use protocol::*;
use std::cmp;
use std::io::Write;

pub(crate) fn write_protocol_client<O: Write>(protocol: Protocol, out: &mut O) -> IOResult<()> {
    // TODO
    Ok(())
}

pub(crate) fn write_protocol_server<O: Write>(protocol: Protocol, out: &mut O) -> IOResult<()> {
    // TODO
    Ok(())
}
