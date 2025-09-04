# CHANGELOG: wayland-server

## Unreleased

## 0.31.10 - 2025-07-28

- Updated Wayland core protocol to 1.24

## 0.31.6 -- 2024-10-23

- Updated Wayland core protocol to 1.23

## 0.31.2 -- 2024-05-30

#### Additions

- Add `Weak::is_alive` allowing to check if a resource is still alive.

## 0.31.1 -- 2024-01-29

- Dropped `nix` dependency in favor of `rustix`

## 0.31.0 -- 2023-09-02

#### Breaking changes

- Bump bitflags to 2.0
- Updated wayland-backend to 0.3
- Use `BorrowedFd<'_>` arguments instead of `RawFd`
- `Resource::destroyed` now passes the resource type instead of the `ObjectId`.

#### Additions

- Add `flush_clients` method to server `DisplayHandle`.
- Implement `AsFd` for `Display` so it can easily be used in a `calloop` source.

#### Bugfixes

- Fixed a lockfile race condition in `ListeningSocket`.

## 0.30.1 -- 30/05/2023

- `New` objects inside `GlobalDispatch` can now have errors posted using `DataInit::post_error`.
- Updated Wayland core protocol to 1.22

## 0.30.0 -- 27/12/2022

## 0.30.0-beta.13

#### Breaking changes

- `Resource::client_id` has been replaced by `Resource::client` making the owning `Client`
   of a `Resource` accessible without a roundtrip to `DisplayHandle`

## 0.30.0-beta.11

#### Bugfixes

- `Weak::upgrade` now checks if the object has been destroyed

## 0.30.0-beta.10

#### Additions

- Introduce `Weak`, a helper type to store resources without risking reference cycles
- Introduce `Proxy::is_alive()` method checking if the protocol object referenced by a resource is still
  alive in the protocol state.

#### Bugfixes

- Ensure that `XDG_RUNTIME_DIR` is an absolute path before trying to use it

## 0.30.0-beta.9

#### Additions

- `Client` is now `Clone`

## 0.30.0-beta.8

#### Breaking changes

- `Display::null_id()` has been removed, instead use `ObjectId::null()`.
- The traits `DelegateDispatch` and `DelegeteGlobalDispatch` have been removed, their functionnality being
  fused into more generic versions of the `Dispatch` and `GlobalDispatch` traits.
- `DisplayHandle::disable_global()` and `DisplayHandle::remove_global()` now require the state type parameter,
  like `create_global`. This is required for compatibility with libwayland 1.21.
- The `socket` module has been flattened into crate root.

#### Additions

- Introduce the `log` cargo feature to control logging behavior.

## 0.30.0-beta.4

#### Breaking changes

- `Resource::post_error` no longer requires a `&mut DisplayHandle`

#### Additions

- Introduce `Resource::client_id`

## 0.30.0-beta.2

#### Breaking changes

- `delegate_dispatch!` can no longer delegate multiple interfaces at once, in order to properly support
  generic delegate base types.

## 0.30.0-beta.1

#### Breaking changes

- Large rework of the API as a consequence of the rework of the backend.

## 0.30.0-alpha10

- Introduce conversion methods between `wayland_backend::Handle` and `DisplayHandle`

## 0.30.0-alpha7

- Introduce `DataInit::custom_init()`

## 0.30.0-alpha5

- Introduce `Display::backend()`

## 0.30.0-alpha4

#### Breaking changes

- The trait `DestructionNotify` is removed, and replaced by a `Dispatch::destroyed()` method.

## 0.30.0-alpha2

#### Breaking changes

- The `DelegateDispatch` mechanism is changed around an explicit trait-base extraction of module
  state from the main compositor state.
- The `DisplayHandle` no longer has a type parameter
- Global manipulation methods are moved from `DisplayHandle` to `Display`

## 0.30.0-alpha1

Full rework of the crate, which is now organized around a trait-based `Dispatch` metchanism.

This can effectively be considered a new crate altogether.
