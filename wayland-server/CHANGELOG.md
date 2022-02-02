# CHANGELOG: wayland-server

## Unreleased

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