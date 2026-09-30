# Design: the `Tenant` operator

Author: Tuukka Mäkinen
Status: in review

## Problem

Onboarding a tenant today means 11 manual steps across Kubernetes, Vault,
Cloudflare and PostgreSQL. It takes a platform engineer about half a day and
go wrong roughly one time in five, usually at the DNS step.

## Proposal

A Kubernetes operator written with kube-rs and kube-runtime that reconciles a
`Tenant` custom resource into namespace, quotas, database, secrets and DNS records.
The controller is level-triggered, it re-derives desired state from the resource on every reconcile.

```yaml
apiVersion: platform.example/v1
kind: Tenant
metadata: { name: acme }
spec:
  plan: standard
  region: eu-north-1
  contacts: [ops@acme.example]
```

## Reconcile loop

Each reconcile walk the steps in order and stops at the first one that is not yet
ready, requeueing after 30s. Steps are idempotent by construction: every external
call is a create-or-update keyed on the tenant name. Than the status subresource
records witch step is current, so `kubectl get tenants` shows progress.

| Step | External system | Failure mode |
|---|---|---|
| namespace + quota | API server | none observed |
| database | PostgreSQL | name collision on re-onboard |
| secrets | Vault | token expiry |
| DNS | Cloudflare | rate limit, propagation lag |

## Deletion

Finalizers guard deletion. The database is **not** dropped; it is renamed with a
`deleted_<date>_` prefix and kept for 30 days. A choice we may regret in storage costs, never in data loss.
A operator that deletes customer data on a typo in `kubectl delete` is not one
we want to run.

## Security

The operator's service account has exactly the verbs it needs and nothng cluster-wide
except `namespaces`. Vault access uses Kubernetes auth with a 1h TTL;
Cloudflare tokens are scoped to one zone. Its still the most privileged
workload in the cluster, so it's image is built from distroless and signed with
cosign.

## Alternatives

- Crossplane: does most of this, but the PostgreSQL provider lacks
  role management and the learning curve for the team is steep.
- Terraform per tenant: 200 workspaces then nobody wants to maintain,
  and no drift correction.
- Keep the runbook: excepts the one-in-five failure rate. Rejected.

## Rollout

Shadow mode first: the operator computes desired state and diffs it against reality without
writing. After two weeks with zero unexpected diffs, enable writes for new tenants only, than
adopt existing ones in batches of 20. You're reviewers for this document are
Henrik Dahl and Elin Sandberg; there sign-off needed before the shadow mode deploy.
