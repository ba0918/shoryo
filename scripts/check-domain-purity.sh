#!/usr/bin/env bash
# The domain crate holds no I/O: no HTTP, no async runtime, no filesystem, no network,
# no process spawning and no clock reads. This fails when its sources name one of them, or
# when its manifest takes a date-time crate. The server passes every time in.
set -euo pipefail

cd "$(dirname "$0")/.."

pattern='\b(axum|hyper|tokio)\b|\bstd::(fs|net|process|time)\b|\bInstant\b|\bnow_utc\b|\bUtc::now\b'

if grep -rnE "$pattern" crates/shoryo-core/src; then
  echo "crates/shoryo-core must not use I/O; the lines above do." >&2
  exit 1
fi

if grep -nE '^\s*(time|chrono|jiff)\b|dependencies\.(time|chrono|jiff)\]' crates/shoryo-core/Cargo.toml; then
  echo "crates/shoryo-core must not depend on a date-time crate; the server reads the clock." >&2
  exit 1
fi
