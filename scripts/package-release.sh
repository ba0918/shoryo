#!/usr/bin/env bash
# Builds the static release binary and packs it for a GitHub release:
# <out>/shoryo-<target>.tar.gz with `shoryo` at its root, and its SHA-256 beside it.
set -euo pipefail

cd "$(dirname "$0")/.."

target="${1:?usage: scripts/package-release.sh <target> <out-dir>}"
out="${2:?usage: scripts/package-release.sh <target> <out-dir>}"

rustup toolchain install
rustup target add "$target"
cargo build --release --locked --target "$target"

mkdir -p "$out"
archive="shoryo-$target.tar.gz"
tar -czf "$out/$archive" -C "target/$target/release" shoryo
(cd "$out" && sha256sum "$archive" > "$archive.sha256")
