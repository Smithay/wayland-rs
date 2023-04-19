# CHANGELOG: wayland-backend

## Unreleased

## 0.1.2 -- 19/04/2023

#### Bugfixes

- In the rust server backend, don't send `delete_id` messages for server-created objects.
- In the system server backend, wakeup the event loop if there are pending destructors waiting
  to be precessed.

## 0.1.1 -- 16/02/2023

#### Bugfixes

- In sys backend, fix global data not being cleaned up on display drop.

## 0.1.0 -- 27/12/2022

## 0.1.0-beta.14

#### Bugfixes

- In rust backend, retry read if message is incomplete, instead of `Malformed` error.

## 0.1.0-beta.10

#### Breaking changes

- `Message` is now also generic on the `Fd` type, and `io_lifetimes::OwnedFd` is used instead of `RawFd` when
  appropriate.

#### Bugfixes

- The rust backend no longer ever does a blocking flush.
- Server-side sys backend is now able to track liveness of external objects.

## 0.1.0-beta.8

#### Breaking changes

- all backends: creating null `ObjectId` is now done through the `ObjectId::null()` method, and the
  `null_id()` methods on the backends are removed.
- `Argument::Str` now contains an `Option`, which is correctly mapped to nullable strings. This fixes
  segfaults that previously occurred dereferencing the null pointer in the system backend.
- server: `disable_global` and `remove_global` now require the state type parameter, like `create_global`.
  This is required for compatibility with libwayland 1.21.
- `ArgumentType::Array` and `ArgumentType::NewId` no longer take a `AllowNull` and are never nullable.

#### Additions

- client: introduce `Backend::dispatch_inner_queue()` meant for ensuring a system backend in guest mode can
  still process events event it does not control reading the socket.
- introduce the `log` cargo feature to control logging behavior
- A dummy implementation of ClientData is now provided through `()` and all trait methods are optional

## 0.1.0-beta.7

#### Bugfixes

- backend/sys: the inner lock is no longer held when destructors are invoked
- backend/sys: sys backend does not abort process when `Backend::disable_global` is invoked more than once

## 0.1.0-beta.6

#### Additions

- client: `ObjectId` now implements the `Hash` trait
- server: `ObjectId`, `ClientId` and `GlobalId` now implement the `Hash` trait

#### Bugfixes

- backend/sys: the inner lock is no longer held when destructors are invoked

## 0.1.0-beta.5

#### Additions

- client/sys: introduce `Backend::from_foreign_display`

#### Bugfixes

- The server backend now correctly associates interfaces with its object arguments when parsing
  messages with nullable object arguments.

## 0.1.0-beta.4

#### Bugfixes

- server-rs: the inner lock is no longer help when destructors are invoked

## 0.1.0-beta.3

#### Bugfixes

- server-sys: Skip unmanaged globals in the global filter (caused a segfault)

## 0.1.0-beta.2

#### Breaking changes

- server-sys: move `display_ptr()` from `Backend` to `Handle`.

## 0.1.0-beta.1

#### Breaking changes

- Both client and server APIs have been profoundly reworked. The backend now has internal locking
  mechanism allowing handles to it to be cloned and shared accross the application.

#### Bugfixes

- Fix a crash when exactly filling the internal buffers.

## 0.1.0-alpha7

#### Bugfixes

- Client-side with the rust backend, `wl_display` events are now properly printed with other events
  when `WAYLAND_DEBUG=1` is set.

## 0.1.0-alpha6

#### Changes

- Server-side, request callbacks are now allowed to omit providing an `ObjectData` for newly
  created objects if they triggered a protocol error

#### Bugfixes

- Fix display leaks on system server backend.
- Fix a panic when trying to send a message with a null object argument followed by a
  non-null object argument
- Fix various memory leaks

## 0.1.0-alpha5

- Expose `wl_display` pointer on system server backend

## 0.1.0-alpha4

#### Breaking changes

- Server-side `ObjectData::destroyed()` now has access to the `&mut D` server-wide data.

## 0.1.0-alpha3

#### Additions

- Introduce `WEnum::into_result` as a convenience method.

## 0.1.0-alpha2

## 0.1.0-alpha1

Initial pre-release of the crate.
