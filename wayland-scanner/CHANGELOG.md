# CHANGELOG: wayland-scanner

## Unreleased

Generated enums now derive `Ord` and `Hash`.

## 0.30.0-alpha1

Full rework of the crate together of the reworks of `wayland-client` and `wayland-server`.

The code generation is now achieved using procedural macros rather than build scripts.