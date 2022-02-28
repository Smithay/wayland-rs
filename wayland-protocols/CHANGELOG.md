# CHANGELOG: wayland-protocols

## Unreleased

### Additions
- Bump wayland-protocols to 1.25
 - A new misc protocol, `org-kde-kwin-idle`.

## 0.30.0-alpha3

### Breaking changes

- Update wlr-protocols
  - `wlr-output-management-unstable-v1` now marks `finished` event as destructor.
  - `wlr-foreign-toplevel-management-unstable-v1` now marks `finished` event as destructor.

### Additions

- Bump wayland-protocols to 1.24
  - A new staging protocol, `drm-lease-v1`.
  - `pointer-gestures-unstable-v1` is now version 3, introducing hold gestures.
  - `linux-dmabuf-unstable-v1` is now version 4, introducing dmabuf feedback.

## 0.30.0-alpha1

Rework of the crate, as a consequence of the reworks of `wayland-client` and `wayland-server`.
