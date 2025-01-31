#!/bin/sh

# Exit on error
set -e

# OUT_DIR is specified by the rust build environment
if [ -z $OUT_DIR ]; then
    echo "OUT_DIR not specified"
    exit 1
fi
BUILD_DIR=$OUT_DIR/rapidsnark
BINARY_PATH=$BUILD_DIR/build/rapidsnark/package/bin

# If binary exists, exit
if [ -e $BINARY_PATH ]; then
    exit 0
fi

rm -rf $BUILD_DIR
git clone https://github.com/iden3/rapidsnark.git $BUILD_DIR
cd $BUILD_DIR
git submodule update --init --recursive