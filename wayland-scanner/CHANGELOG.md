# CHANGELOG: wayland-scanner

## Unreleased

# 0.30.1 -- 2023-06-17

- Generated protocol object types now implement `Borrow<ObjectId>` and `Hash`

## 0.30.0 -- 2022-12-27

## 0.30.0-beta.10

#### Additions

- An `opcode()` method is added to message enums to retrive the opcode associated with a message variant.

## 0.30.0-beta.9

- Migrate from xml-rs to quick-xml

## 0.30.0-beta.6

- Generated enums now derive `Ord` and `Hash`.
- The scanner now generates constants for the opcode values of the protocol messages.

## 0.30.0-alpha1

Full rework of the crate together of the reworks of `wayland-client` and `wayland-server`.

The code generation is now achieved using procedural macros rather than build scripts.
