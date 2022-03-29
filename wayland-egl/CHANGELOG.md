# CHANGELOG: wayland-egl

## Unreleased

#### Breaking changes

- `WlEglSurface` creation API now correcly handles all error conditions, as a result
  `wayland-egl` now has its own error type.

## 0.30.0-alpha6

#### Breaking changes

- `WlEglSurface` is now `!Sync`, as it should have been from the start

## 0.30.0-alpha1

Rework of the crate as a consequence of the rework of `wayland-client`.