# ADR 004: Replace the in-house auth service with an identity provider

Status: accepted
Date: 2026-03-10

## Context

Our in-house auth service handles passwords, sessions, TOTP and SAML for
enterprise customers. It was written in 2019 and has had four maintainers since. The
security review in February found two issues in the password reset flow and one in the
SAML assertion parser. Neither was exploited, but their the kind of bug
we should not be finding ourselves.

## Decision

Adopt a hosted identity provider and delete the in-house service. We chose Auth0 over
Keycloak because we do not want to operate a identity server, and over
Cognito because Cognito's SAML support did not cover encrypted
assertions, which two enterprise customers require.

## Consequences

Users will need to log in again once, because session cookies change format. We will
organize the cutover for a Sunday morning and notify customers a week ahead.

The auth team's roll changes from building to integrating. That is what most of them asked for anyway.

Cost goes from roughly $400/month of compute to about $2,100/month in subscription fees.
Given that the two security findings each cost more than a year of subscription in
engineering time, we consider this a good trade.

## Migration plan

1. Import password hashes with there existing bcrypt cost factor.
2. Point `/login` and `/logout` at the provider, keep `/session` as a shim.
3. Move SAML customers one at a time. Sanna Korhonen owns the customer communication.
4. Delete the old service after the the last customer is migrated.

Step 4 is the one people forget. Put it in the calendar now, do not wait until it feels safe.

## Open questions

- Do we keep a offline copy of password hashes for rollback? Probably not, gross.
- Rate limits on the provider's management API
