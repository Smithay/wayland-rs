# CHANGELOG: wayland-backend

## Unreleased

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
