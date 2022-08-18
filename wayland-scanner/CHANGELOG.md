# CHANGELOG: wayland-scanner

## Unreleased

- Migrate from xml-rs to quick-xml

## 0.30.0-beta.6

- Generated enums now derive `Ord` and `Hash`.
- The scanner now generates constants for the opcode values of the protocol messages.

## 0.30.0-alpha1

Full rework of the crate together of the reworks of `wayland-client` and `wayland-server`.

The code generation is now achieved using procedural macros rather than build scripts.
