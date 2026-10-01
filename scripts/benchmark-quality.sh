#!/usr/bin/env bash
set -euo pipefail

# Build outside the measured process; fixtures and features stay fixed across runs.
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."
export LC_ALL=C
export TZ=UTC
export CARGO_NET_OFFLINE=true

corpus="$(bash scripts/prepare-public-benchmark.sh)"
export EXPLICIT_PUBLIC_CORPUS="$corpus"
devenv shell -- cargo build --locked --offline --release --features voikko --example autoresearch --example autoresearch_public

fixed_output="$(devenv shell -- ./target/release/examples/autoresearch)"
printf '%s\n' "$fixed_output"
public_output="$(devenv shell -- ./target/release/examples/autoresearch_public)"
printf '%s\n' "$public_output"

metric() {
  local key=$1 output=$2 value='' count=0 line
  while IFS= read -r line; do
    if [[ "$line" =~ ^METRIC[[:space:]]$key=([0-9]+([.][0-9]+)?)$ ]]; then
      value=${BASH_REMATCH[1]}
      count=$((count + 1))
    fi
  done <<< "$output"
  [[ "$count" == 1 ]] || { printf 'Missing or duplicate metric: %s\n' "$key" >&2; return 1; }
  printf '%s\n' "$value"
}

fixed_score="$(metric detection_score "$fixed_output")"
public_score="$(metric public_detection_score "$public_output")"
score="$(jq -n --argjson fixed "$fixed_score" --argjson public "$public_score" '($fixed + $public) / 2')"
printf 'METRIC generalization_score=%.6f\n' "$score"
