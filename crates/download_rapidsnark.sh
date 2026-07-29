#!/bin/sh

# Fetch the prebuilt rapidsnark static libraries for $TARGET and leave them,
# flattened, in $OUT_DIR/rapidsnark/$TARGET where build.rs looks for them.
#
# Libraries come from the upstream iden3/rapidsnark GitHub release. Set
# RAPIDSNARK_DOWNLOAD_BASE_URL to pull the same assets from a mirror, or set
# RAPIDSNARK_LIB_DIR (read by build.rs) to use local libraries and skip the
# network altogether.

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

# The archives crates/build.rs links against.
REQUIRED_LIBS="librapidsnark.a libfr.a libfq.a libgmp.a"

ARCH=$(echo "$TARGET" | cut -d'-' -f1)

# Upstream names its assets by platform, not by Rust target triple, so map the
# triple onto an asset. Targets missing from this list fall back to the generic
# Linux build for their architecture, which may not link.
#
# LIPO_ARCH is the Mach-O slice to keep, set only for Apple platforms. Their
# assets ship universal archives, and rustc rejects those outright when
# bundling a static library: the arm64e slice of the iOS libgmp.a and the arm64
# slice of the simulator librapidsnark.a are not themselves valid ar archives,
# so it fails with "Unsupported archive identifier". Reducing each archive to
# the one slice being built for sidesteps that.
LIPO_ARCH=
case "$TARGET" in
    aarch64-apple-ios)
        SLUG=iOS
        LIPO_ARCH=arm64
        ;;
    aarch64-apple-ios-sim)
        SLUG=iOS-Simulator
        LIPO_ARCH=arm64
        ;;
    # librapidsnark.a and libgmp.a in the simulator asset are universal, but
    # libfr.a and libfq.a are built arm64 only, so this target gets as far as
    # the ensure_arch check and then asks for RAPIDSNARK_LIB_DIR.
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
            aarch64 | arm64) SLUG=linux-arm64 ;;
            x86_64 | amd64) SLUG=linux-x86_64 ;;
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

# sha256 of each pinned asset, so a retagged upstream release cannot swap
# binaries underneath us. Regenerate with:
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

# Verify the download against the pinned digest. A host without either checksum
# tool warns rather than fails, so a missing coreutils cannot break the build.
verify_checksum() {
    zip_file="$1"
    expected="$2"

    if command -v shasum > /dev/null 2>&1; then
        actual=$(shasum -a 256 "$zip_file" | cut -d' ' -f1)
    elif command -v sha256sum > /dev/null 2>&1; then
        actual=$(sha256sum "$zip_file" | cut -d' ' -f1)
    else
        echo "warning: neither shasum nor sha256sum is available, skipping checksum verification"
        return 0
    fi

    if [ "$actual" != "$expected" ]; then
        echo "Checksum mismatch for $zip_file"
        echo "  expected $expected"
        echo "  actual   $actual"
        return 1
    fi
}

# Reduce a universal archive to the single slice being built for, and confirm a
# thin archive is already the right architecture. Upstream coverage is uneven
# per library, so checking here turns a silent architecture mismatch (which
# would only surface when an app links the rlib) into a build failure.
ensure_arch() {
    lib_file="$1"
    arch_name="$2"

    # Not Mach-O, so there is nothing to check. ELF assets are single-arch.
    info=$(lipo -info "$lib_file" 2> /dev/null) || return 0

    case "$info" in
        'Architectures in the fat file'*)
            if ! echo "$info" | tr ' ' '\n' | grep -qx "$arch_name"; then
                echo "$(basename "$lib_file") in $ASSET has no $arch_name slice."
                report_missing_arch "$arch_name"
                return 1
            fi
            if ! lipo -thin "$arch_name" -output "$lib_file.thin" "$lib_file"; then
                echo "Failed to extract the $arch_name slice of $lib_file"
                return 1
            fi
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

# Already unpacked by an earlier build in this OUT_DIR. Every library has to be
# there: a partial directory would otherwise be treated as done and linked.
unpacked=yes
for lib in $REQUIRED_LIBS; do
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
# -f so an HTTP error fails here instead of surfacing as a corrupt zip.
if ! curl -fSL -o "$ZIP_FILE" "$BASE_URL/$ASSET"; then
    echo "Failed to download $BASE_URL/$ASSET"
    echo "Set RAPIDSNARK_DOWNLOAD_BASE_URL to use a mirror, or RAPIDSNARK_LIB_DIR to build against local libraries."
    exit 1
fi

if ! verify_checksum "$ZIP_FILE" "$SHA256"; then
    exit 1
fi

EXTRACT_DIR=$BUILD_DIR/extract-$SLUG
rm -rf "$EXTRACT_DIR"
mkdir -p "$EXTRACT_DIR"

echo "Unzipping $ASSET"
if ! unzip -q "$ZIP_FILE" -d "$EXTRACT_DIR"; then
    echo "Failed to unzip $ZIP_FILE"
    exit 1
fi

# Asset layout is not uniform: the iOS and iOS-Simulator zips keep the archives
# at the top level while every other platform nests them under lib/. Copy by
# name so both shapes end up flat in $LIB_DIR.
for lib in $REQUIRED_LIBS; do
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
