# Change Log

## Unreleased

- [client] Fix a potential race when the client destroys an object after pending events
  for it have already been queued in the event queue

## 0.22.0 -- 2019-01-31

- [scanner] Generate `EventHandler` and `RequestHandler` traits for trait-based event
  and request handling (as opposed to manually matching on enums).
- **Breaking** [client/server] Change `NewProxy/NewRequest::implement()` to accept
  a handler struct which implements the corresponding `EventHandler/RequestHandler` trait.
- [client/server] Introduce `NewProxy/NewRequest::implement_closure()` which behaves
  like the previous `NewProxy/NewRequest::implement()` and
  `NewProxy/NewRequest::implement_dummy()` which adds an empty implementation.
- **Breaking** [client/server] `implement()` method will now runtime-track which thread it
  can be called on, making it safe to use non-`Send` implementation. `implement_nonsend()`
  is removed as being non-`Send` is now the default. `implement_threadsafe()` is added
  for when a threadsafe impl is needed.
- **Breaking** [server] When the `native_lib` cargo feature is active, none of the types
  of the server crate are threadsafe, as the underlying C lib actually never supported it.
  The rust implementation remains threadsafe.
- **Breaking** [client/server] `Proxy<I>` and `Resource<I>` are now wrapped in their
  corresponding `I` objects. All `RequestsTrait` traits were removed and methods for
  sending requests and events are now part of the `I` objects themselves. This means that
  it is no longer needed to import all necessary `RequestsTrait` traits into scope and
  deal with their name clashes. This also means that a lot of `Proxy<I>` and `Resource<I>`
  types were replaced with just `I`. To convert between `Proxy/Resource<I>` and `I`,
  `From` implementations are provided (use `.into()`), as well as an
  `AsRef<Proxy/Resource<I>>` implementation for `I`.
- **Breaking** [client/server/commons] `AnonymousObject` was moved out of
  `wayland-commons` into `wayland-client` and `wayland-server`. This is to accommodate the
  design change above.
- [scanner] Fixed a number of cases where invalid Rust code would be generated if variable
  or method names in the protocol were Rust keywords.
- [commons] Properly close FDs we dup-ed after sending them to the server, to avoid leaking
  open FDs.
- [commons/client] Introduce message iterators and message sinks, allowing to opt-in into an
  iterator-based handling of messages.
- [client/server] Fix handling of messages & ID receycling in races around object destruction,
  which could cause `wayland-client` to unexpectedly panic.
- **Breaking** [client/server] Update the core protocol to 1.16
- [protocols] Introduce misc/gtk-primary-selection
- **Breaking** [client] Lifted the thread-safety bound on `GlobalManager`. This
  means that the closure given to `GlobalManager::new_with_cb()` no longer
  needs to be `Send`.
- [client] Added an explicit `.expect()` to binding in the `global_filter!()`
  macro. This gets rid of the warning when using `global_filter!()`.
- [client/server] `WAYLAND_DEBUG` output will now be directed to stderr (as it should have been from the
  start).
- [scanner] Generate `__nonexaustive` variants to protocol enums to match semver guarantees of protocols
- [client] replace `GlobalManager::instantiate_auto` with `GlobalManager::instanciate_range`
- [client/server] Check that versions of messages are correctly respected on objects

## 0.21.11 -- 2019-01-19

- [commons] Fix incorrect number of server-side ids when using the rust implementation
- [protocols] Update wlroots-protocols.
- Implement `std::error::Error` for public error types in all crates
- [scanner] Generate constants in each interface module to expose the minimum version of
  each request and event.

## 0.21.10 -- 2019-01-06

- [client] Undo protocol update which was a breakind change

## 0.21.9 -- 2019-01-05 (yanked)

- [sys] Fix dependencies specification

## 0.21.8 -- 2019-01-05 (yanked)

- Update `mio` dependency
- [server] Fix some leaks when dropping the display when using the rust implementation
- [protocols] Update wayland-protocols and wlroots-protocols.

## 0.21.7 -- 2018-11-28

- [protocols] Only depend on `wayland-sys` if `native_lib` is set
- [server] Resources are now properly marked as dead during client cleanup

## 0.21.6 -- 2018-11-27

- [sys] Introduce `wl_display_destroy_clients` in server-side bindings
- [server] Properly cleanup clients when dropping `WlDisplay` when using `native_lib`.

## 0.21.5 -- 2018-11-27 (yanked)

- [server] Properly cleanup clients when dropping `WlDisplay` when using `native_lib`.

## 0.21.4 -- 2018-11-10

- Dependencies updates

## 0.21.3 -- 2018-11-02

- [protocols] Add bindings to [wlr-protocols](https://github.com/swaywm/wlr-protocols)
- [client & server] Support calloop 0.4 as well as 0.3

## 0.21.2 - 2018-09-27

- [client] Introduce `Proxy::send_constructor` allowing the sending of a message creating a new object
  atomically. This solves races when creating objects from two threads concurrently (#206). Also, this
  simplifies the implementation of `wayland-scanner`, removing some unsafe code from it.
- [client] Unmanaged proxies (when using `native_lib`) are now correctly considered alive rather than dead

## 0.21.1 - 2018-09-25

- [server] When using `native_lib`, only attempt to filter globals that are managed by the rust lib.

## 0.21.0 - 2018-09-11

- [protocols] Remove support for wayland-wall, which has been discontinued.
- [client] Add `Display::get_display_ptr()` to differentiate between the wrapper and the
  actual `wl_display`
- [client] Rework user-data mechanism to introduce type-safety
- [server] Rework `Resource` user-data mechanism to introduce type-safety
- [server] Implement global filtering capabilities, to selectively advertise globals to clients.
  - **Breaking**: this bumps the minimal version of the C libraries to 1.13.0
- [server] **Breaking**: migrate the event loop logic to using `calloop` rather than our own
- [client] Add the `eventloop` cargo feature which provides `calloop` integration for `EventQueue`

## 0.21.0-alpha1 - 2018-07-18

- Complete refactor of the library to integrate a pure rust implementation of the protocol,
  controlled by the `native_lib` switch cargo feature

## 0.20.11 - 2018-07-17

- [scanner] Fixed `*mut_` typo when generating code for nullable array arguments
- [protocols] Integrate `xdg-decoration` protocol to replace KDE's decoration protocol.
- [client] `Proxy` now implements `PartialEq` and `Eq`
- [server] `Resource` now implements `PartialEq` and `Eq`

## 0.20.10 - 2018-06-04

- [client] Fix regression from previous release where `Display` was no longer
  `Send` and `Sync`.

## 0.20.9 - 2018-06-03

- [client] Allow the creation of a `Display` from a foreign `wl_display`
- [protocols] Expose generated C interfaces for protocol interop
- [protocols] Update wayland-protocols to 1.14

## 0.20.8 - 2018-05-22

- [client/server] Use `Clone` trait rather than inherent method for `clone()`.
- [client] GlobalManager is now `Clone`.

## 0.20.7 - 2018-05-20

- [client/server] Bugfix: Actually destroy the proxy/resource when sending a destructor message.

## 0.20.6 - 2018-05-11

- [client] Expose `ConnectError` which should have been public from start.

## 0.20.5 - 2018-05-10

- [client] Fix a typo in the name of `instantiate_*` methods of `GlobalManager`. Old names
  are kept for backward compatibility but marked as deprecated.

## 0.20.4 - 2018-05-08

- [client] Check availability of library in `Display::connect_to_env`

## 0.20.3 - 2018-05-07

- [protocols] Tweak cargo features to avoid always pulling both wayland-client and wayland-server.

## 0.20.2 - 2018-04-30

- [server] Add methods for manual client management

## 0.20.1 - 2018-04-23

- [server] Add a workaround in `Resource` definition to avoid https://github.com/rust-lang/rust/issues/50153

## 0.20.0 - 2018-04-21

- **Breaking** Complete rework of the libraries, basically everything is changed.

## 0.14.1 - 2018-03-09

- [scanner] All objects are implementable (except display), as it is required to properly setup destructors.

## 0.14.0 - 2018-02-21

- **Breaking** [server] Return the implementation data when the creation of an event source fails

## 0.13.0 - 2018-02-21

- [server] Add `with_idata` to event sources to access idata without removing the event source
- **Breaking** [server] Move common event source functions into new `EventSource` trait

## 0.12.5 - 2018-01-02

- [sys] Update lazy_static dependency to 1.0
- [protocols] Add KDE's server decoration protocol

## 0.12.4 - 2018-12-17

- [server] Add a `ptr()` method to `Display` for ffi purpose

## 0.12.3 - 2017-12-06

- [protocols] XDG Shell is now stable

## 0.12.2 - 2017-11-07

- [server] Move `register_global` to `EventLoopHandle`

## 0.12.1 - 2017-11-02

- [server] Add idle event sources
- [sys] More robust loading of wayland-egl and wayland-cursor (failed on ubuntu 17.04)

## 0.12.0 - 2017-10-29

- [protocols] Rework internal structure to reflect versionning of unstable
  protocols (breaking change for unstable protocols)
- [protocols] Add the [wayland-wall](https://github.com/wayland-wall/wayland-wall)
  protocol collection.
- [client] No longer auto-close the connexion on `WlDisplay` drop (this was unsafe)

## 0.11.4 - 2017-10-21

- [scanner] Bugfix: properly destroy implementation data when a destructor
  method is called.
- [server] Bugfix: don't destroy ID of Timer and Signal event sources on drop
- [server] Event sources now return their ID on `destroy()`
- [scanner] Update xml-rs dependency to 0.7

## 0.11.3 - 2017-10-15

- [client] Add `EnvHandler::clone_inner()`

## 0.11.2 - 2017-10-12

- [client] Fields of `EnvNotify` were mistakenly private.

## 0.11.1 - 2017-10-11

- [client] Add `EnvHandler::init_with_notify()` to still be notified about
  global events when using `EnvHandler`.
- [client/server] Externalise state logic to crate `token_store`

## 0.11.0 - 2017-10-09

- **Breaking change**: Update bitflags dependency to 1.0. Generated code for
  protocols will now have bitflags values as associated constants to these
  bitflags structs.

## 0.10.3 - 2017-10-04

- [server] Update nix dependency to 0.9

## 0.10.2 - 2017-09-21

- [server] Add `Resource::same_client_as(..)` for checking if two
  resources are from the same client.

## 0.10.1 - 2017-09-19

- [server] Correct some forgotten stuff in the previous release

## 0.10.0 - 2017-09-19

- **Breaking change**: large rework of the event loops / event queues
  to a new architecture separating logic from data, helping data-sharing
  between different implementations in a same event loop/queue.
- **Breaking change**: event loops / event queues are no longer `Send`, and
  as such can accept non-`Send` data. It is still possible to directly create
  them in different threads, as the `WlDisplay` is `Sync`.

## 0.9.10 - 2017-09-08

- [sys] Print debug msg only when the `WAYLAND_RS_DEBUG` env variable is set
- [client/server] Allow removal of handlers from event queues or event loops
- [server] Fix wrong logic in FD event source causing spurious errors
- [sys] Also try to load libwayland-client.so.0 and libwayland-server.so.0

## 0.9.9 - 2017-06-23

- [scanner] Normalize whitespaces in doc summary (fixes wayland-protocols 0.9.8)

## 0.9.8 - 2017-06-23

- The `declare_handler!(..)` macros can now handle generic types with trait bounds
- [sys] Implement the `wl_signal_*` functions
- [sys] Don't panic if the .so versions are too old and missing symbols
- [protocols] Update to wayland-protocols 1.8

## 0.9.7 - 2017-06-11

- [scanner] Fix objects not being properly destroyed after calling destructor requests
- [protocols] Remove the `nightly` feature, now that rustc's `static_recursion` is stable

## 0.9.6 - 2017-06-01

- Migrate the repository to https://github.com/smithay

## 0.9.5 - 2017-05-31

- [client] Add a method to create `WlEglSurface` from a raw `wl_surface` ptr
- [client] `WlDisplay::get_fs` is unsafe as it should always have been

## 0.9.4 - 2017-04-20

- [server] Fix a memory corruption in global registration
- [scanner] Fix a null-check leading to segfaults

## 0.9.3 - 2017-04-17

- [server] Bugfix previous release...

## 0.9.2 - 2017-04-17

- [server] Fix a bug of register-related functions leading to resources not being
  properly recognized. Thanks @fangyuanziti.

## 0.9.1 - 2017-03-31

- [client] Proxy objects are now cloneable via `Proxy` methods
- [client] impl Debug for RequestResult
- [server] Server objects are noe cloneable via `Resource` methods
- [server] impl Debug for EventResult

## 0.9.0 - 2017-03-19

- [breaking-change] Be more conservative regarding the use of `user_data` from the C libraries.
  This makes us compatible with manipulation of wayland objects managed by other libraries.
  `wayland-client` and `wayland-server` will not attempt to manage objects already managed by
  something else.

## 0.8.7 - 2017-03-15

- [server] Correct secondary event source handlers API

## 0.8.6 - 2017-03-13

- Robustify macros regarding shadowing of `Result` (thanks to @Daggerbot)
- [sys] Fix typos & errors in symbol names (thanks to @jplatte and @drakulix for spotting them)
- [server] Add support for secondary event sources and multiple event loops

## 0.8.4 - 2017-02-19

### Server updates

- Add `resource_is_registered` to check if a given resource is registered to
  a given handler
- Add 'Resource::post_error()` to send protocol errors

### Scanner updates

- `#[derive(PartialEq)]` for enums

## 0.8.1 - 2017-02-19

- Add a missing public import of `Destroy` trait

## 0.8.0 - 2017-02-19

### Scanner updates

- [breaking change] Don't generate result-like return type on proxies that cannot be destroyed

### Sys updates

- [breaking change] Correct argument types to take optionnal `destructor_func_t`

### Server updates

- Add a destructor mechanism for resources

## 0.7.8 - 2017-02-12

- Add a raw user-data mechanism

## 0.7.7 - 2017-01-31

- Improve a client example (thanks @ideasman42)
- Update metadata of the crates on crates.io

## 0.7.6 - 2016-11-12

### Scanner updates

- Properly handle conflicts in bitflags names

### Protocols updates

- Creation of the crate

### Client & Server updates

- expose interface structs for extention protocols integration

## 0.7.5 - 2016-11-08

### Common updates

- Add `declare_delegating_handler!(..)` macro for delegading an handler impl to a field of
  the handler struct
- update `lazy_static` dependency

### Server updates

- Add methods to add socket to the server's event loop

## 0.7.4 - 2016-10-16

### Client upates

- Concurent read API ( EventQueue::prepare_read() and WlDisplay::get_fd() )

## 0.7.3 - 2016-10-08

### Client updates

- Fix multi-queue dispatching (events on other queue than default were not dispatched)

## 0.7.2 - 2016-10-08

### Common updates

- Event queues and event loops are now `Send` and require handlers to be `Send`

### Client updates

- the `cursor` api is now `Send`

### Server updates

- fix a typo in `declare_handler!` macro ( #70 from @fangyuanziti )

## 0.7.1 - 2016-10-02

### Common updates

- Proxies and Resources are nor `Send+Sync` as they should be
- `equals` method to chek if two handles refer to the same wayland object
- `Init` trait allowing handlers to be initialized after insertion in an event queue/loop

### Client updates

- `egl` modules binding to `libwayland-egl` providing OpenGL support
- `cursor` module binding to `libwayland-cursor` giving access to system's cursor theme

## 0.7.0 - 2016-09-27

Complete rewrite of the libs to a new architecture.

Addition of wayland-server to the libs.

## 0.6.2 - 2016-05-29

Add Iterator impl to EventIterator.

## 0.6.1 - 2016-05-29

Fix premature 0.6.0 release

- Add missing ReadEventsGuard public import
- Hide internals details
- Polish the EventIterator API

## 0.6.0 - 2016-05-28 (yanked)

### Internal changes changing the API

- Rework `EventIterator` internals to avoid adding unnecessary overhead
- Fix soundness of destructors
- Integrate referencing enums from other interfaces

### Protocol extensions

- added stable `wp-viewporter`
- added stable `wp-presentation_time`
- added unstable `wpu-xdg_shell`

## 0.5.9 - 2016-02-08

### Changes

- Update `dlib` dependency to v0.3 to match new macro syntax rules.

## 0.5.8 - 2016-01-07

### BugFixes

- Fix typos and missed things introduced in previous version.

## 0.5.7 - 2016-01-06

### Internal Changes

- Do not rely on lib for C types, but rather std::os::raw.
  Should improve soundness in the long term.

## 0.5.6 - 2016-01-03

### Bugfixes

- Stop trying to set the dispatcher on buffers from wayland-cursor.

## 0.5.5 - 2015-12-30

### Added

- Interface to `libwayland_cursor` in `sys` and `client`, behind the
  `cursor` cargo feature.

## 0.5.4 - 2015-12-13

### Bugfixes

- `WlEglSurface` is now `Send` and `Sync` as it should be.

## 0.5.3 - 2015-12-11

### Added

- wayland-client: `ProxyId` is now `Hash`

## 0.5.2 - 2015-12-09

### Bugfixes

- wayland-sys: Remove inexistant `wl_log` symbols from the bindings
- wayland-client: improve `egl_surface_ptr()` method of WlEglSurface

## 0.5.1 - 2015-12-09

### Added

- `is_available()` and `egl::is_available()` functions

## 0.5 - 2015-12-09

First unified version of wayland-sys, wayland-scanner and wayland-client.

### Added

- `CHANGELOG.md`
- Use local versions in travis testing
