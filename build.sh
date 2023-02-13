#!/usr/bin/env bash
set -e
# set -x

# TODO: This should handle more than just building plrustc...

srcroot="$(cd "$(dirname "$0")" > /dev/null && pwd)"
cd "$srcroot"

if [ -z "$CARGO_TARGET_DIR" ]; then
    export CARGO_TARGET_DIR="$srcroot/plrustc/target"
fi
if [ -z "$CARGO" ]; then
    CARGO="cargo"
fi
if [ -z "$RUSTC" ]; then
    RUSTC="rustc"
fi

export RUSTC_BOOTSTRAP=1

version=$($RUSTC --version | cut -d ' ' -f 2)
if [ "$version" != "1.67.1" ]; then
    echo "rustc ('$RUSTC') is not version 1.67.1" >&2
    exit 1
fi

host=$($RUSTC --version --verbose | grep "^host:" | cut -d ' ' -f 2)
sysroot=$($RUSTC --print sysroot)
libdir="$sysroot/lib/rustlib/$host/lib"

if ! [ -d "$libdir" ]; then
    echo "No such directory '$libdir'. Make sure you have rustc-dev installed" >&2
    exit 2
fi

cd "$srcroot/plrustc/plrustc"

# if [ -z "$RUSTFLAGS" ]; then
RUSTFLAGS="-Zunstable-options -Zbinary-dep-depinfo"
# fi

if [ "$NO_RPATH" != "1" ]; then
    RUSTFLAGS="-Clink-args=-Wl,-rpath,$libdir $RUSTFLAGS"
fi

if [ "$SET_SYSROOT" = "1" ]; then
    # Set fallback sysroot if requested.
    export PLRUSTC_SYSROOT="$sysroot"
fi

echo "plrustc build starting..." >&2

env RUSTFLAGS="$RUSTFLAGS" \
     "$CARGO" build --release \
        -p plrustc --bin plrustc \
        --target "$host"

cd "$srcroot"

mkdir -p "$srcroot/build/bin"
cp "$CARGO_TARGET_DIR/$host/release/plrustc" "$srcroot/build/bin/plrustc"

echo "plrustc build completed" >&2
echo "  result binary is located at '$srcroot/build/bin/plrustc'" >&2
