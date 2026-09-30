# ADR 007: Replace RabbitMQ with NATS JetStream for job dispatch

Status: accepted
Date: 2026-05-12
Deciders: Maja Wiklund, Ilkka Saarinen, Diego Ortega

## Context

Job dispatch between the scheduler and 60 worker pods runs on on a three node
RabbitMQ cluster. It works, but every quarter we spend a few days on split-brain
recovery after a network partition, and the Erlang tuning knobs are understood by
exactly one person. Message volume is modest: 2,000 msg/s peak, 4kB average.

## Options

1. Keep RabbitMQ, move to quorum queues. Fixes the partition behaviour but
   not the bus factor.
2. Kafka. Overkill for our volume and it's consumer group rebalancing
   makes short-lived workers painful.
3. NATS JetStream. Single Go binary, Raft replication, work queue
   streams with explicit acks.

## Decision

Adopt JetStream. Three replicas, one stream per job class, `WorkQueuePolicy` retention,
30s ack wait with a maximum of five deliveries before a message land in the
dead letter stream.

## Consequences

Workers loose the AMQP client library and gain a smaller one. The
migation runs both systems in parallel for two weeks with a bridge that
copies every job into JetStream; workers switch over per job class. Rollback is a flag flip, the bridge keeps RabbitMQ current until we delete it.

Ordering guarantees change. RabbitMQ gave us per queue ordering; JetStream gives per subject
ordering, which is stronger then we need. Nothing in the job model depends on
order, and the one report that appeared to was reading `created_at` anyway.

Monitoring moves from the RabbitMQ management UI to Prometheus metrics
exposed by the server. A win, since nobody looked at the management UI outside incidents.

## Risks

- jetstream file storage needs fast disks. We use gp3 volumes at
  6,000 IOPS; the benchmark showed headroom of 10x.
- The team has no production experience with it. Their is a two-day workshop
  booked for June, and Diego ran it at a previous employer.
- Client libraries for Elixir are community maintained. We verfied
  that the one we need supports pull consumers and been released in the last month.

## Rejected alternatives

SQS was a obvious candidate and would have removed the cluster
entirely. It lost on latency: p99 dispatch latency of 120ms versus 4ms, which
matters for the interactive job class. Also, nobody wanted another IAM policy review.
