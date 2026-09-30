# Runbook: draining a Kubernetes node

Use this when a node is unhealthy, needs a kernel update, or is being retired. Draining moves
every pod off the node and marks it unschedulable.

## Before you start

Check what is running there. Stateful workloads need more care ⟪correct|than⟫ stateless ones.

```console
kubectl get pods --all-namespaces --field-selector spec.nodeName=node-14 -o wide
```

If the list ⟦agreement|include|includes⟧ a ⟪product|Kafka⟫ broker or a ⟪product|PostgreSQL⟫ replica, page
the owning team first. Do not drain anyway and hope for the best; that is how we lost a partition in March.

## Draining

```console
kubectl cordon node-14
kubectl drain node-14 --ignore-daemonsets --delete-emptydir-data --grace-period=60
```

⟪code|`--ignore-daemonsets`⟫ is required because ⟪product|DaemonSet⟫ pods cannot be evicted.
⟪code|`--delete-emptydir-data`⟫ acknowledges that ⟪code|`emptyDir`⟫ volumes are lost.
Anything that needed to survive should not have been in ⟪code|`emptyDir`⟫ ⟦missing_extra_word|in first place|in the first place⟧.

The drain respects ⟪product|PodDisruptionBudgets⟫. If a budget blocks eviction,
the command waits. ⟦fragment|Sometimes forever, if the budget is misconfigured.|This can go on forever if the budget is misconfigured.⟧ Check with:

```console
kubectl get pdb --all-namespaces
```

A budget with ⟪code|`ALLOWED DISRUPTIONS`⟫ at 0 will never let ⟦its_its|it's|its⟧ pods go. Talk to the
owner rather ⟪correct|than⟫ deleting the budget. Kubernetes is doing
exactly what it was told to do.

## After maintenance

```console
kubectl uncordon node-14
```

Pods do not move back automatically. ⟪product|Descheduler⟫ runs every ⟪unit|10 minutes⟫ and
rebalances ⟪term|hot⟫ nodes, so ⟦their_there|their|there⟧ is no need to do anything manual unless the
cluster is under ⟦spelling|presure|pressure⟧.

## Retiring a node

After draining, remove the node from the cluster and from the Terraform state:

```console
kubectl delete node node-14
terraform state rm 'module.nodes.aws_instance.node["14"]'
```

Then terminate the instance in the ⟪product|AWS⟫ console. ⟦spelling|Forgeting|Forgetting⟧ the last step
means we pay for ⟦a_an|a idle|an idle⟧ ⟪product|m6i.2xlarge⟫ until someone notices the bill.
⟪informal|Ask me how I know.⟫

## Gotchas

- ⟪list|Cordon first, drain second⟫; otherwise the scheduler may put new pods on the node mid-drain.
- The ⟪unit|60s⟫ grace period is per pod, not total. A node with 40 slow-stopping pods takes a while.
- ⟪product|kubectl⟫ ⟪version|1.29⟫ changed the ⟪code|`--force`⟫ semantics; read the release notes before using it.
