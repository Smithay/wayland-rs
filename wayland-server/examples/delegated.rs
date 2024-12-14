#![allow(clippy::single_match)]

use delegated::{OutputHandler, OutputManagerState};
use wayland_server::Display;

/// A demonstration of how delegateing can make implementing protocols an implementation detail
///
/// Users of this module only need to implement `OutputHandler` trait on their state,
/// Implementations of `GlobalDispatch` and `Dispatch` can remain an internal detail of the module.
///
/// In a way you can pretend that everything inside of this submodule is a library / different crate
mod delegated {
    use wayland_server::{
        backend::GlobalId,
        protocol::wl_output::{self, WlOutput},
        Client, DataInit, Dispatch, DisplayHandle, GlobalDispatch, New,
    };

    pub trait OutputHandler {
        fn state(&mut self) -> &mut OutputManagerState;
        fn some_callback_one_might_need(&mut self);
    }

    pub struct OutputManagerState {
        global_id: GlobalId,
    }

    impl OutputManagerState {
        /// Create a [`WlOutput`] global, and handle it's requests internally
        /// It can use [`OutputHandler`] trait to callback to your `D` state.
        pub fn create_delegated_global<D>(dh: &DisplayHandle) -> Self
        where
            D: OutputHandler + 'static,
        {
            // Let's create a global that dispatches it's bind requests to our
            // `OutputManagerState::bind` rather than to `D`,
            // that way it can remain an implementation detail
            let global_id = dh.create_delegated_global::<D, WlOutput, (), Self>(4, ());
            Self { global_id }
        }

        /// Let's expose global id to the user, so they can remove the global when needed
        pub fn global_id(&self) -> GlobalId {
            self.global_id.clone()
        }
    }

    impl<D: OutputHandler> GlobalDispatch<WlOutput, (), D> for OutputManagerState {
        /// Called whenever a global created via `create_delegated_global<_, WlOutput, _, OutputManagerState>`
        /// gets binded by a client
        fn bind(
            state: &mut D,
            _handle: &DisplayHandle,
            _client: &Client,
            resource: New<WlOutput>,
            _global_data: &(),
            data_init: &mut DataInit<'_, D>,
        ) {
            // Let's init the object as delegated as well, that way it will dispatch it's requests to our `OutputManagerState::request`
            // rather than to `D`
            let _output = data_init.init_delegated::<_, _, Self>(resource, ());

            // Wa can easily callback the `D` from here if needed, eg.
            let _some_state_one_might_need = state.state();
            state.some_callback_one_might_need();
        }
    }

    impl<D: OutputHandler> Dispatch<WlOutput, (), D> for OutputManagerState {
        /// Called whenever an object created via `init_delegated<WlOutput, _, OutputManagerState>`
        /// receives a client request
        fn request(
            state: &mut D,
            _client: &Client,
            _resource: &WlOutput,
            request: wl_output::Request,
            _data: &(),
            _dhandle: &DisplayHandle,
            _data_init: &mut DataInit<'_, D>,
        ) {
            match request {
                wl_output::Request::Release => {
                    // Wa can easily callback the `D` from here if needed, eg.
                    state.some_callback_one_might_need();
                }
                _ => {}
            }
        }
    }
}

struct App {
    output_state: OutputManagerState,
}

impl OutputHandler for App {
    fn state(&mut self) -> &mut OutputManagerState {
        &mut self.output_state
    }

    fn some_callback_one_might_need(&mut self) {}
}

fn main() {
    let display = Display::<App>::new().unwrap();

    // Let's ask `OutputManagerState` to implement `WlOutput` for us, only calling us back whenever
    // necessary via `OutputHandler` trait
    let output_state = OutputManagerState::create_delegated_global::<App>(&display.handle());

    let app = App { output_state };

    display.handle().remove_global::<App>(app.output_state.global_id());
}
