#!/usr/bin/env bash
set -euo pipefail

# Build outside the measured process; fixtures and features stay fixed across runs.
cd -- "$(dirname -- "${BASH_SOURCE[0]}")"
export LC_ALL=C
export TZ=UTC
export CARGO_NET_OFFLINE=true

devenv shell -- cargo build --locked --offline --release --features voikko --example autoresearch
devenv shell -- ./target/release/examples/autoresearch
