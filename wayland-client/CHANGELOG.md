# CHANGELOG: wayland-client

## Unreleased

#### Breaking changes

- Requests that create new objects now produce inert proxies when called on
  objects with invalid IDs instead of failing with `InvalidId`.  This matches
  the behavior of non-object-creating requests (which also ignore the error).

- `Connection::blocking_dispatch` has been removed; use `EventQueue::blocking_dispatch`.

#### Additions

- `QueueFreezeGuard` for avoiding race conditions while constructing objects.

## 0.1.0-beta.8

#### Breaking changes

- `Connection::null_id()` has been removed, instead use `ObjectId::null()`.
- `EventQueue::sync_roundtrip()` has been renamed to `EventQueue::roundtrip()`.
- Module `globals` has been removed as the abstractions it provide are not deemed useful.
- The trait `DelegateDispatch` as been removed, its functionnality being fused into a more generic
  version of the `Dispatch` trait.

#### Additions

- Introduce the `log` cargo feature to control logging behavior

## 0.30.0-beta.6

- Introduce `EventQueue::poll_dispatch_pending` for running dispatch using an async runtime.

## 0.30.0-beta.1

#### Breaking changes

- Large rework of the API as a consequence of the rework of the backend.

## 0.30.0-alpha10

- Introduce conversion methods between `wayland_backend::Handle` and `ConnectionHandle`

## 0.30.0-alpha2

#### Breaking changes

- The `DelegateDispatch` mechanism is changed around an explicit trait-base extraction of module
  state from the main app state.

## 0.30.0-alpha1

Full rework of the crate, which is now organized around a trait-based `Dispatch` metchanism.

This can effectively be considered a new crate altogether.
