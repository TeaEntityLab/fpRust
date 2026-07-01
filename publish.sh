#!/usr/bin/env bash
set -euo pipefail

VERSION="$(grep '^version' Cargo.toml | head -1 | sed -E 's/.*"([^"]+)".*/\1/')"
TAG="v${VERSION}"

if ! git diff --exit-code --quiet || [[ -n "$(git status --porcelain)" ]]; then
  echo "error: working tree is dirty; commit or stash changes before packaging" >&2
  exit 1
fi

echo "Preparing to publish fp_rust ${VERSION} (tag: ${TAG})"

echo "==> cargo package --list"
cargo package --list

echo "==> cargo package (dry-run)"
cargo package --no-verify

if ! git rev-parse "${TAG}" >/dev/null 2>&1; then
  echo "error: git tag ${TAG} does not exist; create it before publishing" >&2
  echo "  git tag -a ${TAG} -m \"Release ${VERSION}\"" >&2
  exit 1
fi

echo "==> cargo publish --dry-run"
cargo publish --dry-run

read -r -p "Publish ${VERSION} to crates.io? [y/N] " confirm
if [[ ! "${confirm}" =~ ^[Yy]$ ]]; then
  echo "Aborted."
  exit 1
fi

cargo publish

echo "Published ${VERSION}."
echo "Push commits and the release tag manually when ready:"
echo "  git push && git push origin ${TAG}"
