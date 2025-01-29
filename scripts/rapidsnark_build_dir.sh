#!/bin/sh

set -e

BUILD_DIR=$(mktemp -d)

git clone https://github.com/chancehudson/rapidsnark.git $BUILD_DIR
cd $BUILD_DIR

git submodule init
git submodule update

build_gmp.sh
