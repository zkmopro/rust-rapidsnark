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
    
    echo "Downloading $target..."
    
    # Download file with error handling
    if ! curl -L -o "$zip_file" "https://rapidsnark.zkmopro.org/$target.zip"; then
        echo "Failed to download $target.zip"
        return 1  # Return failure status
    fi
    
    echo "Unzipping $zip_file..."
    
    # Unzip with error handling
    if ! unzip "$zip_file" -d "$BUILD_DIR"; then
        echo "Failed to unzip $zip_file"
        return 1
    fi
    
    echo "✅ Successfully downloaded and extracted $target.zip"
}

# Try downloading the full target
if ! download_and_unzip "$TARGET"; then
    echo "Retrying with local architecture..."
    
    local_arch=$(echo "$TARGET" | cut -d'-' -f1)
    
    if ! download_and_unzip "$local_arch"; then
        echo "Download failed for both $TARGET and $local_arch"
        exit 1  # Exit the script with failure
    fi
fi
