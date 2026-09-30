# Runbook: draining a Kubernetes node

Use this when a node is unhealthy, needs a kernel update, or is being retired. Draining moves
every pod off the node and marks it unschedulable.

## Before you start

Check what is running there. Stateful workloads need more care than stateless ones.

```console
kubectl get pods --all-namespaces --field-selector spec.nodeName=node-14 -o wide
```

If the list include a Kafka broker or a PostgreSQL replica, page
the owning team first. Do not drain anyway and hope for the best; that is how we lost a partition in March.

## Draining

```console
kubectl cordon node-14
kubectl drain node-14 --ignore-daemonsets --delete-emptydir-data --grace-period=60
```

`--ignore-daemonsets` is required because DaemonSet pods cannot be evicted.
`--delete-emptydir-data` acknowledges that `emptyDir` volumes are lost.
Anything that needed to survive should not have been in `emptyDir` in first place.

The drain respects PodDisruptionBudgets. If a budget blocks eviction,
the command waits. Sometimes forever, if the budget is misconfigured. Check with:

```console
kubectl get pdb --all-namespaces
```

A budget with `ALLOWED DISRUPTIONS` at 0 will never let it's pods go. Talk to the
owner rather than deleting the budget. Kubernetes is doing
exactly what it was told to do.

## After maintenance

```console
kubectl uncordon node-14
```

Pods do not move back automatically. Descheduler runs every 10 minutes and
rebalances hot nodes, so their is no need to do anything manual unless the
cluster is under presure.

## Retiring a node

After draining, remove the node from the cluster and from the Terraform state:

```console
kubectl delete node node-14
terraform state rm 'module.nodes.aws_instance.node["14"]'
```

Then terminate the instance in the AWS console. Forgeting the last step
means we pay for a idle m6i.2xlarge until someone notices the bill.
Ask me how I know.

## Gotchas

- Cordon first, drain second; otherwise the scheduler may put new pods on the node mid-drain.
- The 60s grace period is per pod, not total. A node with 40 slow-stopping pods takes a while.
- kubectl 1.29 changed the `--force` semantics; read the release notes before using it.
