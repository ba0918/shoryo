#!/usr/bin/env bash
# Fails unless the release tag (vX.Y.Z), the canonical version in Cargo.toml
# ([workspace.package] version) and the changelog's heading for it all agree.
set -euo pipefail

cd "$(dirname "$0")/.."

tag="${1:?usage: scripts/check-release-version.sh <tag>}"
version="$(cargo metadata --no-deps --offline --format-version 1 | jq -r '.packages[] | select(.name == "shoryo") | .version')"

if [ "$tag" != "v$version" ]; then
  echo "tag $tag does not match the version in Cargo.toml ($version)" >&2
  exit 1
fi
if ! grep -q "^## \[$version\]" CHANGELOG.md; then
  echo "CHANGELOG.md has no heading for $version" >&2
  exit 1
fi
if ! grep -q "^\[$version\]: " CHANGELOG.md; then
  echo "CHANGELOG.md has no comparison link for $version" >&2
  exit 1
fi
