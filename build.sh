#!/usr/bin/env bash
set -ex

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

host=$($RUSTC --version --verbose | grep "^host:" | cut -d ' ' -f 2)
sysroot=$($RUSTC --print sysroot)
libdir="$sysroot/lib/rustlib/$host/lib"

cd "$srcroot/plrustc/plrustc"

if [ -z "$RUSTFLAGS" ]; then
    RUSTFLAGS="-Zunstable-options -Zbinary-dep-depinfo"
fi

if [ "$NO_RPATH" != "1" ]; then
    RUSTFLAGS="-Clink-args=-Wl,-rpath,$libdir $RUSTFLAGS"
fi

export RUSTFLAGS="$RUSTFLAGS"
"$CARGO" build --release \
    -p plrustc --bin plrustc \
    --target "$host"

