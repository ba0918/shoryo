#!/usr/bin/env bash
# Prints the changelog section of one version, the release's notes.
set -euo pipefail

cd "$(dirname "$0")/.."

version="${1:?usage: scripts/release-notes.sh <version>}"
awk -v heading="## [$version]" '
  index($0, heading) == 1 { printing = 1; next }
  printing && /^## \[/ { exit }
  printing && /^\[[^]]+\]: / { exit }
  printing { print }
' CHANGELOG.md
