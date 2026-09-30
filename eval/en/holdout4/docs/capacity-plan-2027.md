# Capacity plan 2027

This plan describes how the platform's capacity keeps up with next year's growth. It assumes that the number of users grows by 30 % and the number of bookings grows by 40 %.

## Current state

The platform runs on a Kubernetes cluster with 12 nodes. The nodes are 80 % utilised at peak. The database handle today's load, but disk space run out in June, which is the single largest risk.

Last year capacity ran out twice. Both incidents were caused by batch jobs that was scheduled at peak time. The team had not prepared for batch volume grow with the user base.

## Forecast

| Resource | Now | Forecast 12/2027 | Action |
|----------|-----|------------------|--------|
| Nodes | 12 | 18 | Add in March |
| Disk (PostgreSQL) | 1.4 TiB | 2.6 TiB | Expand in May |
| Queue throughput | 2 000/h | 3 500/h | None |
| Object storage | 18 TiB | 30 TiB | Lifecycle rule |

The forecast is based on three years of data. The figures do not include a possible new region that join the platform in the autumn. If it join, the figures double.

## Actions

1. Add six nodes. The cost is 48 000 € a year, which is less then the price of a single outage.
2. Move batch jobs to the night. Jobs that cannot be moved are split into smaller pieces.
3. Lower the disk alert threshold to 70 %. Nobody trusts the 95 % threshold any more.
4. Add an lifecycle rule that moves objects older than a year to cold storage.

The actions do not cost much, but they're combined maintenance window are two hours, which is acceptable on sunday night. Each of the actions has an owner.

## Risks

The biggest risk is not money but time: the lead time for nodes is eight weeks. The vendor has warned that January orders arrives in March at the earliest. The order it's therefore placed as soon as the budget is approved.

The second risk is skills. Only two people knows how to expand the database volume. The procedure are therefore documented in the runbook and rehearsed in staging before it is done in production. The vendor's assessment: "The disk expansion is an online operation and should not require downtime if the replica is promoted first." We do not trust this blindly we test it.

The plan was written by Ville Ranta and Aino Kallas. The steering group decides in December which actions come first; if the budget were cut, the disk expansion take priority over the nodes. Seperate funding for the object-storage change have already been approved.
