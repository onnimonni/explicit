#!/usr/bin/env bash
set -euo pipefail

cd -- "$(dirname -- "${BASH_SOURCE[0]}")"
export LC_ALL=C TZ=UTC CARGO_NET_OFFLINE=true EXPLICIT_PROGRESS=0
# Loopback fixtures must never use an environment-provided proxy.
unset HTTP_PROXY HTTPS_PROXY ALL_PROXY http_proxy https_proxy all_proxy
export NO_PROXY='*' no_proxy='*'
export EXPLICIT_COLD_CORPUS="${EXPLICIT_COLD_CORPUS:-../treat}"

# Compilation is excluded. Each invocation starts a new full-build process with empty
# external caches. No source, diagnostics or cache is written into either repository.
devenv shell -- cargo build --locked --offline --release --config profile.release.strip=false --features harper,voikko,swedish --example autoresearch_cold
devenv shell -- ./target/release/examples/autoresearch_cold
