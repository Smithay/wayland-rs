# CHANGELOG: wayland-protocols

## Unreleased

## 0.31.2 -- 2024-01-29

- Bump wayland-protocols to 1.33
  - `linux-dmabuf-unstable-v1` is now stable
  - `linux-dmabuf-unstable-v1` version 5 requires protocol error for planes with different modifiers.
  - A new staging protocol:
    - `ext-transient-seat-v1`

## 0.31.0 -- 2023-09-02

### Breaking changes

- Bump bitflags to 2.0
- Updated wayland-backend to 0.3

## 0.30.1 -- 2023-07-13

### Additions

- Bump wayland-protocols to 1.32
  - new staging protocols: `foreign_toplevel_list`, `cursor_shape` and `security_context`

## 0.30.0 -- 2022-12-27

## 0.30.0-beta.15

### Additions

- Bump wayland-protocols to 1.31
  - A new staging protocol:
    - `fractional-scale-v1`

## 0.30.0-beta.14

### Additions

- Bump wayland-protocols to 1.30
  - A new staging protocol:
    - `tearing-control-v1`

- Bump wayland-protocols to 1.29
  - `xdg-shell` has some new error values, however the version was not bumped:
    - `xdg_wm_base::Error::Unresponsive`
    - `xdg_surface::Error::InvalidSize`
    - `xdg_toplevel::Error::InvalidSize`
    - `xdg_toplevel::Error::InvalidParent`
  - A some new staging protocols:
    - `ext-idle-notify`
    - `wp-content-type`
    - `xwayland_shell_v1`

## 0.30.0-beta.9

### Additions

- Bump wayland-protocols to 1.26
  - `xdg-shell` is now version 5, introducing wm capabilities.
  - A new staging protocol, `single-pixel-buffer`.
  - Events in the following protocols now have properly labeled destructors:
    - `wp-linux-explicit-synchronization`
    - `wp-presentation-time`
    - `wp-drm-lease`
    - `wp-fullscreen-shell`

## 0.30.0-beta.1

### Breaking Changes

- Complete reorganization of the crate around the `wp`/`xdg`/`ext` categories
- Protocols from other origins than the officiel repository are now split into their own crates

## 0.30.0-alpha6

### Additions

- Bump wayland-protocols to 1.25
- A new staging protocol, `ext-session-lock-v1`.

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
