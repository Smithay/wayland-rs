pub mod client;
pub mod server;

use crate::protocol::{Argument, ArgumentType, Interface, ANONYMOUS_INTERFACE};

pub(crate) fn check_for_signature<Id>(signature: &[ArgumentType], args: &[Argument<Id>]) -> bool {
    if signature.len() != args.len() {
        return false;
    }
    for (typ, arg) in signature.iter().copied().zip(args.iter()) {
        if !arg.get_type().same_type(typ) {
            return false;
        }
    }
    true
}

#[inline]
pub(crate) fn same_interface(a: &'static Interface, b: &'static Interface) -> bool {
    a as *const Interface == b as *const Interface || a.name == b.name
}

#[inline]
#[allow(dead_code)]
pub(crate) fn same_interface_or_anonymous(a: &'static Interface, b: &'static Interface) -> bool {
    same_interface(a, b) || same_interface(a, &ANONYMOUS_INTERFACE)
}
