# ADR 004: Replace the in-house auth service with an identity provider

Status: accepted
Date: 2026-03-10

## Context

Our in-house auth service handles passwords, sessions, ⟪acronym|TOTP⟫ and ⟪acronym|SAML⟫ for
enterprise customers. It was written in 2019 and has had four ⟪term|maintainers⟫ since. The
security review in February found two issues in the password reset flow and one in the
⟪acronym|SAML⟫ assertion parser. Neither was exploited, but ⟦their_there|their|they're⟧ the kind of bug
we should not be finding ourselves.

## Decision

Adopt a hosted identity provider and delete the in-house service. We chose ⟪product|Auth0⟫ over
⟪product|Keycloak⟫ because we do not want to operate ⟦a_an|a identity|an identity⟧ server, and over
⟪product|Cognito⟫ because ⟪product|Cognito⟫'s ⟪acronym|SAML⟫ support did not cover encrypted
assertions, which two enterprise customers require.

## Consequences

Users will need to log in again once, because session cookies change format. We will
organize the cutover for a Sunday morning and notify customers a week ahead.

The auth team's ⟦homophone|roll|role⟧ changes from building to integrating. That is what most of them asked for anyway.

Cost goes from roughly ⟪unit|$400/month⟫ of compute to about ⟪unit|$2,100/month⟫ in subscription fees.
Given that the two security findings each cost more ⟪correct|than⟫ a year of subscription in
engineering time, we consider this a good trade.

## Migration plan

1. Import password hashes with ⟦their_there|there|their⟧ existing ⟪term|bcrypt⟫ cost factor.
2. Point ⟪code|`/login`⟫ and ⟪code|`/logout`⟫ at the provider, keep ⟪code|`/session`⟫ as a shim.
3. Move ⟪acronym|SAML⟫ customers one at a time. ⟪name|Sanna Korhonen⟫ owns the customer communication.
4. Delete the old service after ⟦repeated_word|the the|the⟧ last customer is migrated.

Step 4 is the one people forget. ⟦punctuation|Put it in the calendar now, do not wait until it feels safe.|Put it in the calendar now; do not wait until it feels safe.⟧

## Open questions

- Do we keep ⟦a_an|a offline|an offline⟧ copy of password hashes for rollback? ⟪informal|Probably not, gross.⟫
- ⟪list|Rate limits on the provider's management API⟫
