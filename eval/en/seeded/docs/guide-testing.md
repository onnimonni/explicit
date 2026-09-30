# Testing guide

We have three layers: unit tests next to the code, integration tests in tests/ and a
nightly load test. Most changes need only the first two.

## Unit tests

```console
cargo test --lib
```

They run in under 5s. Use proptest for parsers and anything that touches byte
offsets; hand-written cases miss the intresting inputs. Snapshot tests use
insta; review changes with `cargo insta review` rather than blindly accepting
them, because a changed snapshot is a changed behavior.

## Integration tests

```console
cargo test --test '*'
```

Each test starts a gateway on a ephemeral port with a generated config and one
or more fake upstreams. The helpers in tests/common/mod.rs handle ports, temp directories
and shutdown. Do not hardcode ports, the tests run in parallel.

Network access is not available in CI. If your test needs DNS,
mock it with the `Resolver` trait.

## Load tests

The nightly job runs oha against a gateway on a dedicated c6i.xlarge for
10 minutes and posts p50, p99 and RPS to `#gateway-perf`. A
regression of more than 5% on p99 block the release. Run it
locally with `just load` if you touched the hot path; results on a laptop are noisy but
a a 2x regression still shows.

## What to test

- Every config option, at least once, including its default
- Every error path that a user can hit from the config file
- Header handling with mixed case and duplicates
- Reload while requests are in flight

Do not test private helpers directly if a public path covers them. Tests that
mirror the implementation break on every refactor and there value is close to
zero.

## Flakes

A flaky test blocks everyone. If you cannot fix it within an hour, quarantine it
with `#[ignore = "flaky, see #NNN"]` and open the issue. Ignored tests reviewed
every Monday.

## Coverage

cargo-llvm-cov produces the report. We do not enforce a percentage.
Coverage tells you what is not tested, not what is tested well.
