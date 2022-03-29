# CHANGELOG: wayland-sys

## Unreleased

### Changes

- The `ffi_dispatch!` macro no longer requires a trailing comma when invoking functions without
  any argument.

## 0.30.0-alpha1

### Changes

- Errors when dynamiclaly loading the system libraries are now logged to `log` rather than
  printed using `eprintln!`.