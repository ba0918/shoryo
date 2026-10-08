#!/usr/bin/env bash
# The gates every change must pass: formatting, lints with warnings as errors, tests, and the
# domain crate's purity. CI runs this script; run it locally at the repository root to
# reproduce a CI result.
set -euo pipefail

# Installs the toolchain rust-toolchain.toml pins, which is also the declared rust-version,
# so these gates also check that the MSRV builds.
rustup toolchain install

cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
scripts/check-domain-purity.sh
