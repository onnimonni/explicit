# Contributing

Thanks for taking the time. This document explains how we work so your first
pull request goes smoothly.

## Setup

```console
git clone https://github.com/example/gateway
cd gateway
devenv shell
cargo test
```

devenv installs the pinned Rust toolchain, just and the git hooks. If
you do not want Nix, rust-toolchain.toml tells rustup what to install
and .pre-commit-config.yaml lists the hooks.

## Pull requests

- Keep them small. A 300 line diff gets reviewed today; a 3000 line diff gets reviewed
  eventualy.
- Write the description for someone who was not in the meeting.
- Link the issue. If there is no issue, open one first for anything that changes
  behaviour; typo fixes and docs do not need one.
- One logical change per commit. We squash on merge, so the commit messages within a PR matter
  less then the PR title.

## Style

rustfmt and clippy run in the hook. Do not add `#[allow]` without a comment
explaining why. Prefer a explicit match over `unwrap()` outside tests.

Comments explain why, not what. If the code needs a comment to explain what it does,, rewrite the code.

Error messages are lowercase, no trailing period, and name the thing that failed:
`failed to bind 0.0.0.0:8080: address in use`.

## Tests

Every bug fix comes with a test that fails without the fix. Integration tests live in
tests/ and spin up real upstreams on ephemeral ports; they take about 20s.
Run only the unit tests with `cargo test --lib` while iterating.

Flaky tests are bugs. If you see one, open an issue with the sead printed by the
test harness rather than retrying until it passes.

## Commit messages

Imperative mood, 72 characters, no ticket numbers in the subject. The body explains
the motivation. github links the PR automatically; you do not need to.

## Code of conduct

Be kind. Assume good intent. Disagree with the code, not the person.
If that sounds like a poster in an office kitchen, well, it works.

## Maintainers

Onni Hakala, Linnea Bergström and Tuomas Heikkilä review pull requests. Expect
a answer within two business days.
