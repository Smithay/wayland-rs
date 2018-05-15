//! Event-dispatching machinnery

use wire::Message;

/// Trait for dispatcher objects
pub trait Dispatcher {
    /// Dispatch given message
    fn dispatch(&mut self, msg: Message) -> Result<(), ()>;
}
