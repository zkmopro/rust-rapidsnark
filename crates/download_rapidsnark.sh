#!/bin/sh

# Exit on error
set -e

# OUT_DIR is specified by the rust build environment
if [ -z $OUT_DIR ]; then
    echo "OUT_DIR not specified"
    exit 1
fi
# TARGET is specified by the rust build environment
if [ -z $TARGET ]; then
    echo "TARGET not specified"
    exit 1
fi
BUILD_DIR=$OUT_DIR/rapidsnark
mkdir -p $BUILD_DIR

download_and_unzip() {
    local target="$1"
    local zip_file="$BUILD_DIR/$target.zip"
    curl -L -o $zip_file https://rapidsnark.zkmopro.org/$target.zip
    unzip $zip_file -d $BUILD_DIR
}

download_and_unzip $TARGET
# Check if curl was successful
if [ $? -ne 0 ]; then
    arch=$(echo "$TARGET" | cut -d'-' -f1)
    download_and_unzip $arch
fi
