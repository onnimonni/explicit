#!/usr/bin/env bash
# Regenerate dictionaries/harper/words.tsv.zlib (zlib-compressed TSV: word, dialect and
# word-class bits) from harper-core's curated dictionary, at the version in Cargo.lock.
# Run after bumping harper-core; all builds read it (see src/rules/words.rs).
# Inspect with `zlib-flate -uncompress < dictionaries/harper/words.tsv.zlib | less`.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
VERSION="$(cargo metadata --format-version 1 \
  | jq -r '.packages[] | select(.name == "harper-core") | .version')"
cargo run --quiet --features harper --example export_harper_words -- \
  "$VERSION" dictionaries/harper/words.tsv.zlib
