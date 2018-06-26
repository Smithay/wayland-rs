//! Event-dispatching machinnery

use downcast::Downcast;

use {Implementation, MessageGroup};
use wire::Message;

/// Trait for dispatcher objects
pub trait Dispatcher<Meta>: Downcast {
    /// Dispatch given message
    fn dispatch(&mut self, msg: Message, meta: Meta) -> Result<(), ()>;
}

impl_downcast!(Dispatcher<Meta>);

/// A wrapper for turning an Implementation into a Dispatcher
///
/// The created Dispatcher will attempt to parse the messages using
/// `MessageGroup::from_raw`. If successful, it is them transferred to the
/// underlying implementation. If not, returns `Err(())`
pub struct ImplDispatcher<Msg, Meta, Impl>
where Impl: Implementation<Meta, Msg> + 'static,
      Meta: 'static,
      Msg: MessageGroup + 'static
{
    _msg: ::std::marker::PhantomData<&'static Msg>,
    _meta: ::std::marker::PhantomData<&'static Meta>,
    implem: Impl
}

impl<Msg, Meta, Impl> ImplDispatcher<Msg, Meta, Impl>
where Impl: Implementation<Meta, Msg> + 'static,
      Meta: 'static,
      Msg: MessageGroup + 'static
{
    pub fn new(implem: Impl) -> ImplDispatcher<Msg, Meta, Impl> {
        ImplDispatcher {
            _msg: ::std::marker::PhantomData,
            _meta: ::std::marker::PhantomData,
            implem: implem
        }
    }
}
