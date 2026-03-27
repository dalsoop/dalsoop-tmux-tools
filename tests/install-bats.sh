#!/bin/sh
set -eu

PREFIX="${1:-/usr/local}"
BATS_LIBDIR="${BATS_LIBDIR:-/usr/lib/bats}"

apt_get_install() {
    apt-get update
    apt-get install -y git
}

if command -v apt-get >/dev/null 2>&1; then
    apt_get_install
fi

tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT

git clone --depth 1 https://github.com/bats-core/bats-core.git "$tmpdir/bats-core"
"$tmpdir/bats-core/install.sh" "$PREFIX"

mkdir -p "$BATS_LIBDIR"
git clone --depth 1 https://github.com/bats-core/bats-support.git "$BATS_LIBDIR/bats-support"
git clone --depth 1 https://github.com/bats-core/bats-assert.git "$BATS_LIBDIR/bats-assert"
