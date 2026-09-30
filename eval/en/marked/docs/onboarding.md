# Onboarding: platform team

Welcome. This page is the checklist for ⟪correct|your⟫ first two weeks. Your buddy is
listed in the calendar invite; ask them anything, ⟦british|realise|realize⟧ that nobody expects you
to know our stack yet.

## Day 1

- ⟪list|Laptop, accounts, `#platform` and `#platform-oncall` channels⟫
- ⟪list|Clone the `backend` monorepo; a full clone is 1.2GB, use `--filter=blob:none`⟫
- ⟪list|`devenv shell` and `cargo test` in `gateway/`⟫
- Read the four ⟪acronym|ADR⟫s in ⟪path|docs/adr/⟫. They explain most of the architecture
  decisions people will otherwise assume you know.

## Week 1

Ship something small. The ⟪code|`good-first-issue`⟫ label on ⟦capitalization|github|GitHub⟧ is curated
by ⟪name|Linnea Bergström⟫ and each issue has ⟪correct|an estimate⟫ and a pointer into the code.

Shadow the on-call. You will not be paged, but you will sit in on the incident channel and the
handover. This is the fastest way to learn where the bodies are buried.

## Week 2

Pick up a real ticket with ⟪correct|your⟫ buddy. Pair on the design, then work alone and
ask for review. ⟦punctuation|Review turnaround is two business days, ping if it is slower.|Review turnaround is two business days; ping if it is slower.⟧

By the end of week 2 you should be able to deploy the gateway to staging on your own. The
deployment guide has the commands; the ⟪code|`staging`⟫ ⟦capitalization|kubernetes|Kubernetes⟧ context
is in the ⟪product|1Password⟫ vault.

## Things people wish they had known

- The monorepo ⟪acronym|CI⟫ only builds crates you touched. If ⟦your_youre|you're|your⟧ change is green
  locally but red in CI, you probably changed a shared crate and broke a dependent one.
- ⟪product|sccache⟫ ⟦agreement|make|makes⟧ builds fast. If it is slow, check that ⟪code|`SCCACHE_BUCKET`⟫
  is set; ⟪product|devenv⟫ does it for you, a plain shell does not.
- We favor small ⟪acronym|PR⟫s, plain English commit messages and ⟦repeated_word|no no|no⟧
  surprises in production.
- Time zones: the team spans ⟪name|Helsinki⟫, ⟪name|Göteborg⟫ and Lisbon. Core hours are
  ⟪unit|10:00⟫ to ⟪unit|15:00⟫ ⟪acronym|CET⟫.

## Vocabulary

| Term | Meaning |
|---|---|
| ⟪table|gateway⟫ | ⟪table|the edge proxy, this team's main product⟫ |
| ⟪table|the bundle⟫ | ⟪table|the trust bundle ConfigMap with our root certificates⟫ |
| ⟪table|SEV1⟫ | ⟪table|customer facing outage⟫ |
| ⟪table|the laptop⟫ | ⟪table|the offline CA machine in the safe⟫ |

If someone uses a word not in this table, ask. ⟦its_its|Its|It's⟧ probably missing from the table
because everyone forgot it was jargon.
