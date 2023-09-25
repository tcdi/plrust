#!/usr/bin/env bash
set -e
# set -x

abs_path() {
    local path="$1"
    (unset CDPATH && cd "$path" > /dev/null && pwd)
}

script_root="$(abs_path "$(dirname "$0")")"
repo_root="$(abs_path "$script_root/..")"

cd "$repo_root"

if [ -z "$CARGO_TARGET_DIR" ]; then
    export CARGO_TARGET_DIR="$repo_root/plrustc/target"
fi
if [ -z "$CARGO" ]; then
    CARGO="cargo"
fi
if [ -z "$RUSTC" ]; then
    RUSTC="rustc"
fi

export RUSTC_BOOTSTRAP=1

version=$($RUSTC --version | cut -d ' ' -f 2)
if [ "$version" != "1.72.1" ]; then
    echo "rustc ('$RUSTC') is not version 1.72.1" >&2
    exit 1
fi

host=$($RUSTC --version --verbose | grep "^host:" | cut -d ' ' -f 2)
sysroot=$($RUSTC --print sysroot)
libdir="$sysroot/lib/rustlib/$host/lib"

if ! [ -d "$libdir" ]; then
    echo "No such directory '$libdir'. Make sure you have rustc-dev installed" >&2
    exit 2
fi

cd "$repo_root/plrustc/plrustc"

# if [ -z "$RUSTFLAGS" ]; then
RUSTFLAGS="-Zunstable-options -Zbinary-dep-depinfo"
# fi


if [ "$NO_RPATH" != "1" ]; then
    # This produces a portable binary assuming that the rust toolchain location
    # will not not moved, which is reasonable when using rustup, during
    # development, and probably in some production usage. For final install,
    # we'll want to have an option to do the fully correct rpath dance, with
    # `$ORIGIN` and `-z,origin` and similar shenanigans (with completely
    # different names) on macOS. In such a case, we should consider installing
    # to the same directory as e.g. rustc and/or cargo-clippy
    RUSTFLAGS="-Clink-args=-Wl,-rpath,$libdir $RUSTFLAGS"
fi

if [ "$SET_SYSROOT" = "1" ]; then
    # Set fallback sysroot if requested.
    export PLRUSTC_SYSROOT="$sysroot"
fi

echo "plrustc build starting..." >&2


if [[ "$1" = "install" ]]; then
    env RUSTFLAGS="$RUSTFLAGS" \
        "$CARGO" install \
            --path . \
            --target "$host"

    echo "plrustc installation (with 'cargo install') completed" >&2
else
    env RUSTFLAGS="$RUSTFLAGS" \
        "$CARGO" build --release \
            -p plrustc --bin plrustc \
            --target "$host"

    cd "$repo_root"

    mkdir -p "$repo_root/build/bin"
    cp "$CARGO_TARGET_DIR/$host/release/plrustc" "$repo_root/build/bin/plrustc"

    echo "plrustc build completed" >&2
    echo "  result binary is located at '$repo_root/build/bin/plrustc'" >&2
fi
