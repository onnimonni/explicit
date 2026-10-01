#!/usr/bin/env bash
# Keep frozen public README bytes outside the repository, keyed by annotation hash.
set -euo pipefail

fail() {
  printf 'prepare-public-benchmark: %s\n' "$*" >&2
  exit 1
}

[[ $# -le 1 ]] || fail 'usage: prepare-public-benchmark.sh [corpus-directory]'
for tool in jq curl shasum; do
  type -P "$tool" >/dev/null || fail "required executable not found: $tool"
done
JQ=$(type -P jq)
CURL=$(type -P curl)
SHASUM=$(type -P shasum)
ROOT=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)
ANNOTATIONS="$ROOT/eval/public/annotations.json"

sha256() {
  local digest
  digest=$("$SHASUM" -a 256 < "$1") || return
  printf '%s\n' "${digest%% *}"
}

annotation_sha=$(sha256 "$ANNOTATIONS")
# Validate all download inputs before touching the cache or making requests.
"$JQ" -e '
  .version == 1 and
  (.sources | type == "array" and length == 47) and
  ([.sources[].id] | length == (unique | length)) and
  all(.sources[];
    (.id | type == "string" and test("^[A-Za-z0-9][A-Za-z0-9_-]*$")) and
    (.sha256 | type == "string" and test("^[0-9a-f]{64}$")) and
    (.raw_url | type == "string" and
      test("^https://raw\\.githubusercontent\\.com/[A-Za-z0-9][A-Za-z0-9_.-]*/[A-Za-z0-9][A-Za-z0-9_.-]*/[0-9a-f]{40}/[A-Za-z0-9_.-]+(/[A-Za-z0-9_.-]+)*$") and
      (split("/")[6:] | all(. != "." and . != ".."))))
' "$ANNOTATIONS" >/dev/null || fail 'invalid frozen source metadata'
source_rows=$("$JQ" -r '.sources[] | [.id, .raw_url, .sha256] | @tsv' "$ANNOTATIONS")
[[ $(sha256 "$ANNOTATIONS") == "$annotation_sha" ]] || fail 'annotations changed while reading metadata'

corpus=${1:-${TMPDIR:-/tmp}/explicit-public-benchmark-$annotation_sha}
[[ -n "$corpus" ]] || fail 'corpus directory must not be empty'
[[ "$corpus" == /* ]] || corpus="$PWD/$corpus"
# Resolve existing ancestors, then normalize the remaining path before mkdir.
# This also prevents a user-supplied path or symlink from placing data in ROOT.
ancestor=$corpus
suffix=
while [[ ! -d "$ancestor" ]]; do
  suffix="$(basename -- "$ancestor")/$suffix"
  ancestor=$(dirname -- "$ancestor")
done
corpus="$(cd -- "$ancestor" && pwd -P)/$suffix"
IFS=/ read -r -a components <<< "$corpus"
corpus=
for component in "${components[@]}"; do
  case "$component" in
    ''|.) ;;
    ..) corpus=${corpus%/*} ;;
    *) corpus="$corpus/$component" ;;
  esac
done
corpus=${corpus:-/}
case "$corpus" in
  "$ROOT"|"$ROOT"/*) fail 'corpus directory must be outside the repository' ;;
esac
umask 077
mkdir -p -- "$corpus"
corpus=$(cd -- "$corpus" && pwd -P)
case "$corpus" in
  "$ROOT"|"$ROOT"/*) fail 'corpus directory must be outside the repository' ;;
esac

download=
trap '[[ -z "$download" ]] || rm -f -- "$download"' EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
while IFS=$'\t' read -r id raw_url expected_sha; do
  destination="$corpus/$id.md"
  [[ ! -L "$destination" ]] || fail "refusing symlink: $destination"
  if [[ -e "$destination" && ! -f "$destination" ]]; then
    fail "not a regular corpus file: $destination"
  fi
  if [[ -f "$destination" ]]; then
    actual_sha=$(sha256 "$destination") || fail "cannot hash cached source: $id"
    [[ "$actual_sha" != "$expected_sha" ]] || continue
    printf 'prepare-public-benchmark: replacing invalid cached source %s\n' "$id" >&2
  fi
  printf 'prepare-public-benchmark: fetching %s\n' "$id" >&2
  download=$(mktemp "$corpus/.download-$id.XXXXXX")
  # No redirects, curl configuration, or non-HTTPS protocols are permitted.
  "$CURL" --disable --fail --silent --show-error --proto '=https' \
    --connect-timeout 15 --max-time 120 --output "$download" \
    --url "$raw_url" || fail "download failed: $id"
  actual_sha=$(sha256 "$download") || fail "cannot hash downloaded source: $id"
  [[ "$actual_sha" == "$expected_sha" ]] || fail "SHA-256 mismatch for source: $id"
  mv -f -- "$download" "$destination"
  download=
done <<< "$source_rows"
[[ $(sha256 "$ANNOTATIONS") == "$annotation_sha" ]] || fail 'annotations changed while preparing corpus'
printf '%s\n' "$corpus"
