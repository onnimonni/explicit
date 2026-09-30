# Onboarding guide for new engineers

Welcome to the platform team. This guide explains what happens in you're first week and where to find help.

## Day one

You get a laptop, accounts and a YubiKey. Sign in to GitHub and Slack with your work address and join #platform and #incidents. The team has lunch together at 11:30; on Mondays the company pays.

The development environment are set up with Nix. Run `nix develop` in the repository root and you get Go, PostgreSQL and kubectl in the right versions. If Docker misbehaves on macOS, ask Ville; the fix is usually a restart.

```bash
git clone git@github.com:example/platform.git
cd platform && nix develop
make test
```

## Week one

1. Read the architecture overview and the two most recent postmortems. Each of them takes about twenty minutes.
2. Make a small change: fix a typo or add a test. Open a pull request and ask for review.
3. Shadow the on-call engineer for a day. That is where you learn the most.
4. Join Thursday's planning meeting.

Pull requests is reviewed within a day. The reviewers comment on GitHub; do not take comments personally. The code is reviewed, not it's author. A reviewer who have questions asks them in the thread rather then in a direct message.

## Ways of working

- Branches is named `feature/…` and `fix/…`. The branch are deleted after the merge.
- Every change to production go through Argo CD. Nobody runs `kubectl apply` by hand.
- Documentation are written in English; comments and commit messages are too.
- Secrets live in Vault, never in the repository. A number of leaked keys were found last year, and the cost of rotating them was real.

The team's principle is that a mistake is a learning opportunity. Postmortems are blameless. As Mika says: "kaikki on joskus pudottanu tuotantokannan", which means everyone has dropped the production database at some point.

## Where to get help

| Topic | Who |
|-------|-----|
| Accounts and hardware | IT support, #it-support |
| Architecture | Aino Kallas |
| Database | Väinö Linna |
| On-call | the week's on-call engineer, see Opsgenie |

After a month their are a feedback conversation with your manager. Tell them what were unclear in this guide, but do not wait until then if something are wrong. Most of questions is answered fastest in #platform, where the people who work remotely also hang out. The onboarding buddy programme runs in Finnish as well; definately ask if you prefer it.
