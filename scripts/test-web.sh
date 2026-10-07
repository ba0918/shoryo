#!/usr/bin/env bash
# The browser tests of the screen: builds the binary, then runs Playwright against it.
# Run at the repository root; extra arguments go to `playwright test`.
set -euo pipefail

# The toolchain rust-toolchain.toml pins, as scripts/check.sh installs it.
rustup toolchain install
cargo build --locked
npx playwright test "$@"
