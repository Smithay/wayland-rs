# Change Log

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
