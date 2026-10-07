#!/usr/bin/env bash
# The domain crate holds no I/O: no HTTP, no async runtime, no filesystem, no network,
# no process spawning and no clock reads. This fails when its sources name one of them.
set -euo pipefail

cd "$(dirname "$0")/.."

pattern='\b(axum|hyper|tokio)\b|\bstd::(fs|net|process)\b|\bstd::time::SystemTime\b'

if grep -rnE "$pattern" crates/shoryo-core/src; then
  echo "crates/shoryo-core must not use I/O; the lines above do." >&2
  exit 1
fi
