#!/bin/sh
sed -i.bak 's/wayland-sys = { version = "[0-9\.\^]*"/wayland-sys = { path = "..\/wayland-sys"/' ./wayland-client/Cargo.toml
sed -i.bak 's/wayland-scanner = { version = "[0-9\.\^]*"/wayland-scanner = { path = "..\/scanner"/' ./wayland-client/Cargo.toml
