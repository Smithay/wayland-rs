# CHANGELOG: wayland-server

## Unreleased

#### Breaking changes

- Bump bitflags to 2.0
- Updated wayland-backend to 0.2

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
