# Runbook: TLS certificate rotation

Most certificates are issued by ⟪product|cert-manager⟫ and renew themselves 30 days before
expiry. This runbook covers the ones that do not: the ⟪acronym|mTLS⟫ root, the ⟪acronym|SAML⟫
signing certificate and the two customer-pinned leaf certificates.

## Schedule

| Certificate | Lifetime | Rotate at |
|---|---|---|
| ⟪code|`internal-root`⟫ | ⟪unit|5 years⟫ | ⟪table|4 years⟫ |
| ⟪code|`saml-signing`⟫ | ⟪unit|2 years⟫ | ⟪table|18 months⟫ |
| ⟪code|`pinned-*`⟫ | ⟪unit|1 year⟫ | ⟪table|60 days before expiry⟫ |

The calendar reminders live in the platform team's shared calendar. Do not
rely on ⟪correct|your⟫ memory.

## Rotating the internal root

The root rotation is ⟦a_an|an two|a two⟧ phase process. First distribute the new root to every
trust store alongside the old one, ⟪correct|then⟫ reissue leaves, then remove the old root.
Skipping the first phase breaks every ⟪acronym|mTLS⟫ connection at once.

1. Generate the new root on the offline laptop. The ⟪path|/opt/ca/README⟫ file on that laptop has the
   exact ⟪product|openssl⟫ commands.
2. Add the new root to the ⟪code|`trust-bundle`⟫ ⟪product|ConfigMap⟫ in every cluster.
3. Wait for a full rollout. ⟪product|kubectl⟫ ⟦agreement|show|shows⟧ the restart progress with
   ⟪code|`kubectl rollout status`⟫.
4. Switch the issuer. From now on new leaves ⟦agreement|is|are⟧ signed by the new root.
5. After every leaf has been reissued (max ⟪unit|72h⟫), remove the old root from the bundle.

⟦fragment|Between steps 4 and 5, a window of up to three days where both roots are trusted.|Between steps 4 and 5 there is a window of up to three days where both roots are trusted.⟧
This is intended.

## Rotating the SAML signing certificate

Enterprise customers pin this certificate in ⟪correct|their⟫ identity provider. Rotating it
means every customer must update ⟦their_there|they're|their⟧ configuration, so we announce the new
certificate 60 days ahead and publish both in the metadata ⟪acronym|XML⟫ during the overlap.

Customers that ⟦spelling|havent|haven't⟧ switched by the deadline get a reminder ⟪unit|14 days⟫ and
⟪unit|3 days⟫ before. ⟪informal|After that they break, and honestly that is on them.⟫ Account
managers know this and have been told to warn customers early.

## Pinned leaf certificates

Two customers pin our leaf certificate for ⟪url|api.acme.example⟫ instead of the chain. They
need the new certificate ⟦homophone|to|too⟧, before we deploy it. The contact list is in the
⟪product|1Password⟫ vault ⟪code|`platform-shared`⟫.

## Verifying

```console
openssl s_client -connect api.acme.example:443 -servername api.acme.example </dev/null 2>/dev/null | openssl x509 -noout -dates
```

If ⟪code|`notAfter`⟫ has not moved, the load balancer is still serving the old
certificate. It caches for up to ⟪unit|10 minutes⟫.
