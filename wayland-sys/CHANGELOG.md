# CHANGELOG: wayland-sys

## Unreleased

## 0.30.1

#### Bugfixes

- Fix UB in `rust_listener_create`

## 0.30.0

## 0.30.0-beta.10

#### Bugfixes

- Server-side, fix the prototype of `wl_resource_add_destroy_listener`

## 0.30.0-alpha10

#### Changes

- The `ffi_dispatch!` macro no longer requires a trailing comma when invoking functions without
  any argument.

## 0.30.0-alpha1

#### Changes

- Errors when dynamiclaly loading the system libraries are now logged to `log` rather than
  printed using `eprintln!`.
