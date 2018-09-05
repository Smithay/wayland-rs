#!/bin/bash

# download and compile the wayland libs for given version, they will be installed in ~/install

# early exit if the file is already here, we have a cache
clientlib="$HOME/install/lib/libwayland-client.so.0.3.0"
serverlib="$HOME/install/lib/libwayland-server.so.0.1.0"
if [ -f "$clientlib" -a -f "$serverlib" ]; then
    echo "Cache is present, not rebuilding the wayland libs."
    exit 0
fi

wayland_version=$1

mkdir ~/temp/ ~/install

# download and extract
cd ~/temp/
wget https://github.com/wayland-project/wayland/archive/${wayland_version}.tar.gz -O wayland.tar.gz
tar xf wayland.tar.gz
cd wayland-${wayland_version}

# compile and install
./autogen.sh --prefix=$HOME/install --disable-documentation --disable-dtd-validation --disable-dependency-tracking
make
make install
