# CHANGELOG: wayland-backend

## Unreleased

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
