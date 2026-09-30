# ADR 007: Replace RabbitMQ with NATS JetStream for job dispatch

Status: accepted
Date: 2026-05-12
Deciders: ⟪name|Maja Wiklund⟫, ⟪name|Ilkka Saarinen⟫, Diego Ortega

## Context

Job dispatch between the ⟪product|scheduler⟫ and ⟪unit|60⟫ worker pods runs ⟦repeated_word|on on|on⟧ a three node
⟪product|RabbitMQ⟫ cluster. It works, but every quarter we spend a few days on ⟪term|split-brain⟫
recovery after a network partition, and the ⟪product|Erlang⟫ tuning knobs are understood by
exactly one person. Message volume is modest: ⟪unit|2,000 msg/s⟫ peak, ⟪unit|4kB⟫ average.

## Options

1. Keep RabbitMQ, move to quorum queues. Fixes the partition ⟦british|behaviour|behavior⟧ but
   not the bus factor.
2. ⟪product|Kafka⟫. Overkill for our volume and ⟦its_its|it's|its⟧ consumer group rebalancing
   makes short-lived workers painful.
3. ⟪product|NATS JetStream⟫. Single ⟪product|Go⟫ binary, ⟪term|Raft⟫ replication, work queue
   streams with explicit acks.

## Decision

Adopt JetStream. Three replicas, one stream per job class, ⟪code|`WorkQueuePolicy`⟫ retention,
⟪unit|30s⟫ ack wait with a maximum of five deliveries before a message ⟦agreement|land|lands⟧ in the
dead letter stream.

## Consequences

Workers ⟦homophone|loose|lose⟧ the ⟪acronym|AMQP⟫ client library and gain a smaller one. The
⟦spelling|migation|migration⟧ runs both systems in parallel for two weeks with a bridge that
copies every job into JetStream; workers switch over per job class. ⟦punctuation|Rollback is a flag flip, the bridge keeps RabbitMQ current until we delete it.|Rollback is a flag flip; the bridge keeps RabbitMQ current until we delete it.⟧

Ordering guarantees change. RabbitMQ gave us per queue ordering; JetStream gives per subject
ordering, which is stronger ⟦then_than|then|than⟧ we need. Nothing in the job model depends on
order, and the one report that appeared to was reading ⟪code|`created_at`⟫ anyway.

Monitoring moves from the RabbitMQ management ⟪acronym|UI⟫ to ⟪product|Prometheus⟫ metrics
exposed by the server. ⟦fragment|A win, since nobody looked at the management UI outside incidents.|That is a win, since nobody looked at the management UI outside incidents.⟧

## Risks

- ⟦capitalization|jetstream|JetStream⟧ file storage needs fast disks. We use ⟪product|gp3⟫ volumes at
  ⟪unit|6,000 IOPS⟫; the benchmark showed headroom of ⟪unit|10x⟫.
- The team has no production experience with it. ⟦their_there|Their|There⟧ is a two-day workshop
  booked for June, and ⟪name|Diego⟫ ran it at a previous employer.
- Client libraries for ⟪product|Elixir⟫ are community maintained. We ⟦spelling|verfied|verified⟧
  that the one we need supports pull consumers ⟦missing_extra_word|and been|and has been⟧ released in the last month.

## Rejected alternatives

⟪product|SQS⟫ was ⟦a_an|a obvious|an obvious⟧ candidate and would have removed the cluster
entirely. It lost on latency: ⟪unit|p99⟫ dispatch latency of ⟪unit|120ms⟫ versus ⟪unit|4ms⟫, which
matters for the interactive job class. ⟪informal|Also, nobody wanted another IAM policy review.⟫
