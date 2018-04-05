#!/bin/sh

# download and compile the wayland libs for given version, they will be installed in ~/install

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
