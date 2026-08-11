#!/usr/bin/env bash
# Build the Verilator release rustdv's runtime VPI loader is verified against.
# Installs into a caller-owned prefix, so CI and containers need no sudo.
set -euo pipefail

VERSION=5.050
SHA256=ec6723f30c1798b1fbbbed97364f09c431fb4875577c314f37240e99b60a4a04
PREFIX="${1:-/tmp/rustdv-$(id -u)/verilator-$VERSION}"

if [ -x "$PREFIX/bin/verilator" ]; then
    installed="$($PREFIX/bin/verilator --version | awk '{print $2}')"
    if [ "$installed" = "$VERSION" ]; then
        echo "Verilator $VERSION already installed at $PREFIX"
        exit 0
    fi
fi

missing=()
for tool in curl tar autoconf make flex bison perl python3; do
    command -v "$tool" >/dev/null 2>&1 || missing+=("$tool")
done
if [ "${#missing[@]}" -gt 0 ]; then
    echo "Verilator build dependencies missing: ${missing[*]}" >&2
    exit 2
fi

SCRATCH="/tmp/rustdv-$(id -u)/verilator-install"
ARCHIVE="$SCRATCH/verilator-v$VERSION.tar.gz"
mkdir -p "$SCRATCH" "$PREFIX"

if [ ! -f "$ARCHIVE" ]; then
    curl --fail --location --silent --show-error \
        "https://github.com/verilator/verilator/archive/refs/tags/v$VERSION.tar.gz" \
        --output "$ARCHIVE"
fi

if command -v shasum >/dev/null 2>&1; then
    actual="$(shasum -a 256 "$ARCHIVE" | awk '{print $1}')"
elif command -v sha256sum >/dev/null 2>&1; then
    actual="$(sha256sum "$ARCHIVE" | awk '{print $1}')"
else
    echo "Verilator installer needs shasum or sha256sum" >&2
    exit 2
fi
if [ "$actual" != "$SHA256" ]; then
    echo "Verilator archive checksum mismatch: got $actual, expected $SHA256" >&2
    exit 1
fi

SOURCE="$(mktemp -d "$SCRATCH/source.XXXXXX")"
trap 'rm -rf "$SOURCE"' EXIT
tar -xzf "$ARCHIVE" -C "$SOURCE" --strip-components=1

jobs="${VERILATOR_BUILD_JOBS:-2}"
(
    cd "$SOURCE"
    autoconf
    ./configure --prefix="$PREFIX"
    make -j "$jobs"
    make install
)

"$PREFIX/bin/verilator" --version
