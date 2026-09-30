# Onboarding: platform team

Welcome. This page is the checklist for your first two weeks. Your buddy is
listed in the calendar invite; ask them anything, realise that nobody expects you
to know our stack yet.

## Day 1

- Laptop, accounts, `#platform` and `#platform-oncall` channels
- Clone the `backend` monorepo; a full clone is 1.2GB, use `--filter=blob:none`
- `devenv shell` and `cargo test` in `gateway/`
- Read the four ADRs in docs/adr/. They explain most of the architecture
  decisions people will otherwise assume you know.

## Week 1

Ship something small. The `good-first-issue` label on github is curated
by Linnea Bergström and each issue has an estimate and a pointer into the code.

Shadow the on-call. You will not be paged, but you will sit in on the incident channel and the
handover. This is the fastest way to learn where the bodies are buried.

## Week 2

Pick up a real ticket with your buddy. Pair on the design, then work alone and
ask for review. Review turnaround is two business days, ping if it is slower.

By the end of week 2 you should be able to deploy the gateway to staging on your own. The
deployment guide has the commands; the `staging` kubernetes context
is in the 1Password vault.

## Things people wish they had known

- The monorepo CI only builds crates you touched. If you're change is green
  locally but red in CI, you probably changed a shared crate and broke a dependent one.
- sccache make builds fast. If it is slow, check that `SCCACHE_BUCKET`
  is set; devenv does it for you, a plain shell does not.
- We favor small PRs, plain English commit messages and no no
  surprises in production.
- Time zones: the team spans Helsinki, Göteborg and Lisbon. Core hours are
  10:00 to 15:00 CET.

## Vocabulary

| Term | Meaning |
|---|---|
| gateway | the edge proxy, this team's main product |
| the bundle | the trust bundle ConfigMap with our root certificates |
| SEV1 | customer facing outage |
| the laptop | the offline CA machine in the safe |

If someone uses a word not in this table, ask. Its probably missing from the table
because everyone forgot it was jargon.
