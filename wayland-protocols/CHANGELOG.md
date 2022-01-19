# CHANGELOG: wayland-protocols

## Unreleased

### Additions

- Bump wayland-protocols to 1.24
  - A new staging protocol, `drm-lease-v1`.
  - `pointer-gestures-unstable-v1` is now version 3, introducing hold gestures.
  - `linux-dmabuf-unstable-v1` is now version 4, introducing dmabuf feedback.

## 0.30.0-alpha1

Rework of the crate, as a consequence of the reworks of `wayland-client` and `wayland-server`.