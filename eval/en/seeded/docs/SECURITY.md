# Security policy

## Supported versions

| Version | Supported |
|---|---|
| 2.4.x | yes |
| 2.3.x | security fixes only |
| < 2.3 | no |

## Reporting a vulnerability

Email security@acme.example. Our PGP key is at https://acme.example/.well-known/pgp.txt.
Please do not open a public issue, and please do not test against production systems.

We acknowledge reports within 2 business days and aim to ship a fix within 30 days
for high severity issues. You will hear from us at every step; their is no black hole.

## What counts

In scope:

- Authentication and authorization bypass
- Request smuggling or header injection through the proxy
- Memory safety issues reachable from the network
- Dependency vulnerabilities with a demonstrated impact

Out of scope:

- Reports from automated scanners without a proof of concept
- Missing security headers on the marketing site
- Denial of service through resource exhaustion on a default config (rate limits are
  the operator's job and are documented)

## Disclosure

We favour coordinated disclosure. Once a fix is released we publish an advisory on
github with credit to the reporter unless they ask otherwise. If we
cannot agree on a timeline, 90 days from the initial report is the default.

## Bounties

There is no formal bounty programm, but we send swag and, for principle
findings, a thank-you payment at our discretion. We are a small team, be patient with us.

## Hardening advice for operators

- Run the binary as an unprivileged user. The systemd unit in contrib/ does this.
- Put the admin port behind mTLS or bind it to `127.0.0.1`.
- Rotate JWT signing keys at least every 90 days; the process is
  in the operations guide.
- Read the changelog before upgrading. Security fixes are marked and its entries
  say what an attacker could do.
