# Runbook: TLS certificate rotation

Most certificates are issued by cert-manager and renew themselves 30 days before
expiry. This runbook covers the ones that do not: the mTLS root, the SAML
signing certificate and the two customer-pinned leaf certificates.

## Schedule

| Certificate | Lifetime | Rotate at |
|---|---|---|
| `internal-root` | 5 years | 4 years |
| `saml-signing` | 2 years | 18 months |
| `pinned-*` | 1 year | 60 days before expiry |

The calendar reminders live in the platform team's shared calendar. Do not
rely on your memory.

## Rotating the internal root

The root rotation is an two phase process. First distribute the new root to every
trust store alongside the old one, then reissue leaves, then remove the old root.
Skipping the first phase breaks every mTLS connection at once.

1. Generate the new root on the offline laptop. The /opt/ca/README file on that laptop has the
   exact openssl commands.
2. Add the new root to the `trust-bundle` ConfigMap in every cluster.
3. Wait for a full rollout. kubectl show the restart progress with
   `kubectl rollout status`.
4. Switch the issuer. From now on new leaves is signed by the new root.
5. After every leaf has been reissued (max 72h), remove the old root from the bundle.

Between steps 4 and 5, a window of up to three days where both roots are trusted.
This is intended.

## Rotating the SAML signing certificate

Enterprise customers pin this certificate in their identity provider. Rotating it
means every customer must update they're configuration, so we announce the new
certificate 60 days ahead and publish both in the metadata XML during the overlap.

Customers that havent switched by the deadline get a reminder 14 days and
3 days before. After that they break, and honestly that is on them. Account
managers know this and have been told to warn customers early.

## Pinned leaf certificates

Two customers pin our leaf certificate for api.acme.example instead of the chain. They
need the new certificate to, before we deploy it. The contact list is in the
1Password vault `platform-shared`.

## Verifying

```console
openssl s_client -connect api.acme.example:443 -servername api.acme.example </dev/null 2>/dev/null | openssl x509 -noout -dates
```

If `notAfter` has not moved, the load balancer is still serving the old
certificate. It caches for up to 10 minutes.
