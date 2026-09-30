# Design: the `Tenant` operator

Author: ⟪name|Tuukka Mäkinen⟫
Status: in review

## Problem

Onboarding a tenant today means ⟪unit|11⟫ manual steps across ⟪product|Kubernetes⟫, ⟪product|Vault⟫,
⟪product|Cloudflare⟫ and ⟪product|PostgreSQL⟫. It takes a platform engineer about half a day and
⟦agreement|go|goes⟧ wrong roughly one time in five, usually at the ⟪acronym|DNS⟫ step.

## Proposal

A ⟪product|Kubernetes⟫ operator written with ⟪crate|kube-rs⟫ and ⟪crate|kube-runtime⟫ that reconciles a
⟪code|`Tenant`⟫ custom resource into namespace, quotas, database, secrets and ⟪acronym|DNS⟫ records.
⟦punctuation|The controller is level-triggered, it re-derives desired state from the resource on every reconcile.|The controller is level-triggered; it re-derives desired state from the resource on every reconcile.⟧

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

Each reconcile ⟦agreement|walk|walks⟧ the steps in order and stops at the first one that is not yet
ready, requeueing after ⟪unit|30s⟫. Steps are ⟪term|idempotent⟫ by construction: every external
call is a create-or-update keyed on the tenant name. ⟦then_than|Than|Then⟧ the status subresource
records ⟦homophone|witch|which⟧ step is current, so ⟪code|`kubectl get tenants`⟫ shows progress.

| Step | External system | Failure mode |
|---|---|---|
| ⟪table|namespace + quota⟫ | ⟪table|API server⟫ | ⟪table|none observed⟫ |
| ⟪table|database⟫ | ⟪table|PostgreSQL⟫ | ⟪table|name collision on re-onboard⟫ |
| ⟪table|secrets⟫ | ⟪table|Vault⟫ | ⟪table|token expiry⟫ |
| ⟪table|DNS⟫ | ⟪table|Cloudflare⟫ | ⟪table|rate limit, propagation lag⟫ |

## Deletion

Finalizers guard deletion. The database is **not** dropped; it is renamed with a
⟪code|`deleted_<date>_`⟫ prefix and kept for ⟪unit|30 days⟫. ⟦fragment|A choice we may regret in storage costs, never in data loss.|This is a choice we may regret in storage costs, never in data loss.⟧
⟦a_an|A operator|An operator⟧ that deletes customer data on a typo in ⟪code|`kubectl delete`⟫ is not one
we want to run.

## Security

The operator's service account has exactly the verbs it needs and ⟦spelling|nothng|nothing⟧ cluster-wide
except ⟪code|`namespaces`⟫. Vault access uses ⟪term|Kubernetes auth⟫ with a ⟪unit|1h⟫ ⟪acronym|TTL⟫;
⟪product|Cloudflare⟫ tokens are scoped to one zone. ⟦its_its|Its|It's⟧ still the most privileged
workload in the cluster, so ⟦its_its|it's|its⟧ image is built from ⟪product|distroless⟫ and signed with
⟪product|cosign⟫.

## Alternatives

- ⟪product|Crossplane⟫: does most of this, but the ⟪product|PostgreSQL⟫ provider lacks
  role management and the learning curve for the team is steep.
- ⟪product|Terraform⟫ per tenant: ⟪unit|200⟫ workspaces ⟦then_than|then|than⟧ nobody wants to maintain,
  and no drift correction.
- Keep the runbook: ⟦homophone|excepts|accepts⟧ the one-in-five failure rate. Rejected.

## Rollout

Shadow mode first: the operator computes desired state and diffs it against reality without
writing. After two weeks with zero unexpected diffs, enable writes for new tenants only, ⟦then_than|than|then⟧
adopt existing ones in batches of ⟪unit|20⟫. ⟦your_youre|You're|Your⟧ reviewers for this document are
⟪name|Henrik Dahl⟫ and ⟪name|Elin Sandberg⟫; ⟦their_there|there|their⟧ sign-off ⟦missing_extra_word|needed before|is needed before⟧ the shadow mode deploy.
