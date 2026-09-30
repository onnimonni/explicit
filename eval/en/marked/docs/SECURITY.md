# Security policy

## Supported versions

| Version | Supported |
|---|---|
| ⟪version|2.4.x⟫ | ⟪table|yes⟫ |
| ⟪version|2.3.x⟫ | ⟪table|security fixes only⟫ |
| ⟪version|< 2.3⟫ | ⟪table|no⟫ |

## Reporting a vulnerability

Email ⟪url|security@acme.example⟫. Our ⟪acronym|PGP⟫ key is at ⟪url|https://acme.example/.well-known/pgp.txt⟫.
Please do not open a public issue, and please do not test against production systems.

We acknowledge reports within ⟪unit|2 business days⟫ and aim to ship a fix within ⟪unit|30 days⟫
for high severity issues. You will hear from us at every step; ⟦their_there|their|there⟧ is no black hole.

## What counts

In scope:

- ⟪list|Authentication and authorization bypass⟫
- ⟪list|Request smuggling or header injection through the proxy⟫
- ⟪list|Memory safety issues reachable from the network⟫
- ⟪list|Dependency vulnerabilities with a demonstrated impact⟫

Out of scope:

- ⟪list|Reports from automated scanners without a proof of concept⟫
- ⟪list|Missing security headers on the marketing site⟫
- ⟪list|Denial of service through resource exhaustion on a default config⟫ (rate limits are
  the operator's job and are documented)

## Disclosure

We ⟦british|favour|favor⟧ coordinated disclosure. Once a fix is released we publish an advisory on
⟦capitalization|github|GitHub⟧ with credit to the reporter unless they ask otherwise. If we
cannot agree on a timeline, ⟪unit|90 days⟫ from the initial report is the default.

## Bounties

There is no formal bounty ⟦spelling|programm|program⟧, but we send swag and, for ⟦homophone|principle|principal⟧
findings, a thank-you payment at our discretion. ⟦punctuation|We are a small team, be patient with us.|We are a small team; be patient with us.⟧

## Hardening advice for operators

- Run the binary as ⟪correct|an unprivileged⟫ user. The systemd unit in ⟪path|contrib/⟫ does this.
- Put the admin port behind ⟪acronym|mTLS⟫ or bind it to ⟪code|`127.0.0.1`⟫.
- Rotate ⟪acronym|JWT⟫ signing keys at least every ⟪unit|90 days⟫; the process is
  in the operations guide.
- Read the changelog before upgrading. Security fixes are marked and ⟪correct|its⟫ entries
  say what an attacker could do.
