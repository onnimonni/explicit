# Contributing

Thanks for taking the time. This document explains how we work so ⟪correct|your⟫ first
pull request goes smoothly.

## Setup

```console
git clone https://github.com/example/gateway
cd gateway
devenv shell
cargo test
```

⟪product|devenv⟫ installs the pinned ⟪product|Rust⟫ toolchain, ⟪product|just⟫ and the git hooks. If
you do not want ⟪product|Nix⟫, ⟪path|rust-toolchain.toml⟫ tells ⟪product|rustup⟫ what to install
and ⟪path|.pre-commit-config.yaml⟫ lists the hooks.

## Pull requests

- Keep them small. A ⟪unit|300 line⟫ diff gets reviewed today; a ⟪unit|3000 line⟫ diff gets reviewed
  ⟦spelling|eventualy|eventually⟧.
- Write the description for someone who was not in the meeting.
- Link the issue. If ⟪correct|there⟫ is no issue, open one first for anything that changes
  ⟦british|behaviour|behavior⟧; typo fixes and docs do not need one.
- One logical change per commit. We squash on merge, so the commit messages within a PR matter
  less ⟦then_than|then|than⟧ the PR title.

## Style

⟪product|rustfmt⟫ and ⟪product|clippy⟫ run in the hook. Do not add ⟪code|`#[allow]`⟫ without a comment
explaining why. Prefer ⟦a_an|a explicit|an explicit⟧ match over ⟪code|`unwrap()`⟫ outside tests.

Comments explain why, not what. ⟦punctuation|If the code needs a comment to explain what it does,, rewrite the code.|If the code needs a comment to explain what it does, rewrite the code.⟧

Error messages are lowercase, no trailing period, and name the thing that failed:
⟪code|`failed to bind 0.0.0.0:8080: address in use`⟫.

## Tests

Every bug fix comes with a test that fails without the fix. Integration tests live in
⟪path|tests/⟫ and spin up real upstreams on ⟪term|ephemeral ports⟫; they take about ⟪unit|20s⟫.
Run only the unit tests with ⟪code|`cargo test --lib`⟫ while iterating.

Flaky tests are bugs. If you see one, open an issue with the ⟦spelling|sead|seed⟧ printed by the
test harness rather ⟪correct|than⟫ retrying until it passes.

## Commit messages

Imperative mood, 72 characters, no ticket numbers in the subject. The body explains
the motivation. ⟦capitalization|github|GitHub⟧ links the PR automatically; you do not need to.

## Code of conduct

Be kind. Assume good intent. ⟪informal|Disagree with the code, not the person.⟫
⟪informal|If that sounds like a poster in an office kitchen, well, it works.⟫

## Maintainers

⟪name|Onni Hakala⟫, ⟪name|Linnea Bergström⟫ and ⟪name|Tuomas Heikkilä⟫ review pull requests. Expect
⟦a_an|a answer|an answer⟧ within two business days.
