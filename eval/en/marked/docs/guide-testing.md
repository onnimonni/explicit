# Testing guide

We have three layers: unit tests next to the code, integration tests in ⟪path|tests/⟫ and a
nightly load test. Most changes need only the first two.

## Unit tests

```console
cargo test --lib
```

They run in under ⟪unit|5s⟫. Use ⟪product|proptest⟫ for parsers and anything that touches byte
offsets; hand-written cases miss the ⟦spelling|intresting|interesting⟧ inputs. Snapshot tests use
⟪product|insta⟫; review changes with ⟪code|`cargo insta review`⟫ rather than blindly accepting
them, because a changed snapshot is a changed behavior.

## Integration tests

```console
cargo test --test '*'
```

Each test starts a gateway on ⟦a_an|a ephemeral|an ephemeral⟧ port with a generated config and one
or more fake upstreams. The helpers in ⟪path|tests/common/mod.rs⟫ handle ports, temp directories
and shutdown. ⟦punctuation|Do not hardcode ports, the tests run in parallel.|Do not hardcode ports; the tests run in parallel.⟧

Network access is not available in ⟪acronym|CI⟫. If ⟪correct|your⟫ test needs ⟪acronym|DNS⟫,
mock it with the ⟪code|`Resolver`⟫ trait.

## Load tests

The nightly job runs ⟪product|oha⟫ against a gateway on a dedicated ⟪product|c6i.xlarge⟫ for
⟪unit|10 minutes⟫ and posts ⟪unit|p50⟫, ⟪unit|p99⟫ and ⟪unit|RPS⟫ to ⟪code|`#gateway-perf`⟫. A
regression of more than ⟪unit|5%⟫ on ⟪unit|p99⟫ ⟦agreement|block|blocks⟧ the release. Run it
locally with ⟪code|`just load`⟫ if you touched the hot path; results on a laptop are noisy but
⟦repeated_word|a a|a⟧ ⟪unit|2x⟫ regression still shows.

## What to test

- ⟪list|Every config option, at least once, including its default⟫
- ⟪list|Every error path that a user can hit from the config file⟫
- ⟪list|Header handling with mixed case and duplicates⟫
- ⟪list|Reload while requests are in flight⟫

Do not test private helpers directly if a public path covers them. Tests that
mirror the implementation break on every refactor and ⟦their_there|there|their⟧ value is close to
zero.

## Flakes

A flaky test blocks everyone. If you cannot fix it within an hour, quarantine it
with ⟪code|`#[ignore = "flaky, see #NNN"]`⟫ and open the issue. ⟦missing_extra_word|Ignored tests reviewed|Ignored tests are reviewed⟧
every Monday.

## Coverage

⟪product|cargo-llvm-cov⟫ produces the report. We do not enforce a percentage.
⟪informal|Coverage tells you what is not tested, not what is tested well.⟫
