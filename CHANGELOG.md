# Change Log

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

- Add a destructor mechanism for ressources

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
