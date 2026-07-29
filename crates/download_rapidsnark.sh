#!/bin/sh

# Exit on error
set -e

# OUT_DIR is specified by the rust build environment
if [ -z "$OUT_DIR" ]; then
    echo "OUT_DIR not specified"
    exit 1
fi
# TARGET is specified by the rust build environment
if [ -z "$TARGET" ]; then
    echo "TARGET not specified"
    exit 1
fi

RAPIDSNARK_VERSION=v0.0.8
BASE_URL=${RAPIDSNARK_DOWNLOAD_BASE_URL:-https://github.com/iden3/rapidsnark/releases/download/$RAPIDSNARK_VERSION}

BUILD_DIR=$OUT_DIR/rapidsnark
LIB_DIR=$BUILD_DIR/$TARGET

REQUIRED_LIBS="librapidsnark.a libfr.a libfq.a libgmp.a"

ARCH=$(echo "$TARGET" | cut -d'-' -f1)

# Apple assets are universal archives, which rustc rejects when bundling a static
# library ("Unsupported archive identifier"), so thin them to LIPO_ARCH.
#
# build.rs asks for both static and dylib, which rustc resolves to one
# -lrapidsnark after -lstdc++. GNU ld cannot satisfy that from the archive, so
# Linux also needs SHARED_LIB. https://github.com/zkmopro/rust-rapidsnark/pull/11
LIPO_ARCH=
SHARED_LIB=
case "$TARGET" in
    aarch64-apple-ios)
        SLUG=iOS
        LIPO_ARCH=arm64
        ;;
    aarch64-apple-ios-sim)
        SLUG=iOS-Simulator
        LIPO_ARCH=arm64
        ;;
    # Rejected by ensure_arch: the simulator libfr.a and libfq.a are arm64 only.
    x86_64-apple-ios)
        SLUG=iOS-Simulator
        LIPO_ARCH=x86_64
        ;;
    aarch64-apple-darwin)
        SLUG=macOS-arm64
        LIPO_ARCH=arm64
        ;;
    x86_64-apple-darwin)
        SLUG=macOS-x86_64
        LIPO_ARCH=x86_64
        ;;
    aarch64-linux-android) SLUG=android-arm64 ;;
    x86_64-linux-android) SLUG=android-x86_64 ;;
    *)
        case "$ARCH" in
            aarch64 | arm64)
                SLUG=linux-arm64
                SHARED_LIB=librapidsnark.so
                ;;
            x86_64 | amd64)
                SLUG=linux-x86_64
                SHARED_LIB=librapidsnark.so
                ;;
            *)
                echo "No rapidsnark prebuilt covers target $TARGET (architecture $ARCH)."
                echo "Set RAPIDSNARK_LIB_DIR to a directory containing $REQUIRED_LIBS to build against your own."
                exit 1
                ;;
        esac
        echo "No prebuilt matches $TARGET exactly, falling back to the generic $SLUG build."
        ;;
esac

ASSET=rapidsnark-$SLUG-$RAPIDSNARK_VERSION.zip

# Regenerate with:
#   gh api repos/iden3/rapidsnark/releases/tags/$RAPIDSNARK_VERSION \
#     --jq '.assets[] | "\(.name) \(.digest)"'
case "$SLUG" in
    iOS) SHA256=2015bb8825f37356f70aa1a3a7fd66e08bb1205a174d3793bfdf388a9e7e6cfd ;;
    iOS-Simulator) SHA256=6455825c56b03a9001a99aae303541f7565e4587b023223a5e4b148edf203cfa ;;
    macOS-arm64) SHA256=dbd2c1498663223232f9c3ad02259d2839e62e784e9b1f6a0e9bd5070443990d ;;
    macOS-x86_64) SHA256=df116044e6edfd409aa198a9bc828f0f038ddfeaa3243d350460da3a08376631 ;;
    android-arm64) SHA256=2e306704fe1b900261006ec41a7bef92bb3f94839c9989eff5058b133f5dc642 ;;
    android-x86_64) SHA256=966d0e572963af1ff35faa56a5a16754a67000308d0cd89ed6c4d60a3080a2e0 ;;
    linux-arm64) SHA256=704dfbaa6847d4ddf5f63bf7bc8d3e59f007c33e2d8ab16b318090d671253dbd ;;
    linux-x86_64) SHA256=2ec59e3aa5ff498e862d60b3b7abdcd094ea484271750ec1ea14fb7c1305e423 ;;
esac

verify_checksum() {
    zip_file="$1"
    expected="$2"

    if command -v shasum > /dev/null 2>&1; then
        actual=$(shasum -a 256 "$zip_file" | cut -d' ' -f1)
    else
        actual=$(sha256sum "$zip_file" | cut -d' ' -f1)
    fi

    if [ "$actual" != "$expected" ]; then
        echo "Checksum mismatch for $zip_file"
        echo "  expected $expected"
        echo "  actual   $actual"
        return 1
    fi
}

# Upstream coverage is uneven per library, so reject a mismatch here rather than
# leave it to surface when an app links the rlib.
ensure_arch() {
    lib_file="$1"
    arch_name="$2"

    info=$(lipo -info "$lib_file" 2> /dev/null) || return 0

    case "$info" in
        'Architectures in the fat file'*)
            if ! echo "$info" | tr ' ' '\n' | grep -qx "$arch_name"; then
                echo "$(basename "$lib_file") in $ASSET has no $arch_name slice."
                report_missing_arch "$arch_name"
                return 1
            fi
            lipo -thin "$arch_name" -output "$lib_file.thin" "$lib_file" || return 1
            mv "$lib_file.thin" "$lib_file"
            ;;
        *)
            actual=${info##*: }
            if [ "$actual" != "$arch_name" ]; then
                echo "$(basename "$lib_file") in $ASSET is $actual, but $TARGET needs $arch_name."
                report_missing_arch "$arch_name"
                return 1
            fi
            ;;
    esac
}

report_missing_arch() {
    echo "Upstream does not publish an $1 build of every library in $ASSET."
    echo "Set RAPIDSNARK_LIB_DIR to a directory holding $1 builds of $REQUIRED_LIBS to build this target."
}

# Every library, so an interrupted build cannot look finished.
unpacked=yes
for lib in $REQUIRED_LIBS $SHARED_LIB; do
    if [ ! -f "$LIB_DIR/$lib" ]; then
        unpacked=no
        break
    fi
done
if [ "$unpacked" = yes ]; then
    exit 0
fi
rm -rf "$LIB_DIR"

mkdir -p "$LIB_DIR"
ZIP_FILE=$BUILD_DIR/$ASSET

echo "Downloading $ASSET from $BASE_URL"
if ! curl -fSL -o "$ZIP_FILE" "$BASE_URL/$ASSET"; then
    echo "Failed to download $BASE_URL/$ASSET"
    echo "Set RAPIDSNARK_DOWNLOAD_BASE_URL to use a mirror, or RAPIDSNARK_LIB_DIR to build against local libraries."
    exit 1
fi

verify_checksum "$ZIP_FILE" "$SHA256"

EXTRACT_DIR=$BUILD_DIR/extract-$SLUG
rm -rf "$EXTRACT_DIR"
mkdir -p "$EXTRACT_DIR"

echo "Unzipping $ASSET"
unzip -q "$ZIP_FILE" -d "$EXTRACT_DIR"

# The iOS assets keep these at the top level, other platforms nest them in lib/.
for lib in $REQUIRED_LIBS $SHARED_LIB; do
    src=$(find "$EXTRACT_DIR" -type f -name "$lib" | head -1)
    if [ -z "$src" ]; then
        echo "$ASSET does not contain $lib"
        rm -rf "$LIB_DIR" "$EXTRACT_DIR"
        exit 1
    fi
    cp "$src" "$LIB_DIR/$lib"

    if [ -n "$LIPO_ARCH" ] && ! ensure_arch "$LIB_DIR/$lib" "$LIPO_ARCH"; then
        rm -rf "$LIB_DIR" "$EXTRACT_DIR"
        exit 1
    fi
done

rm -rf "$EXTRACT_DIR" "$ZIP_FILE"

echo "rapidsnark libraries for $TARGET ready in $LIB_DIR"
