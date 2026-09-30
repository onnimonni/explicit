# Runbook: message queue backlog

Use this runbook when Grafana fires `queue_depth > 50000` or when the on-call engineer notices that the number of unacknowledged messages is growing for more than ten minutes.

## First five minutes

1. Acknowledge the alert in Opsgenie so that the other engineers knows someone is looking.
2. Check the consumer count with `rabbitmqctl list_queues name consumers`.
3. Confirm that the list of consumers matches the expected deployment size.

If the consumer count is zero, the consumers has crashed; go to section 3. If the count is normal but the messages are still piling up, one of the consumers is probably stuck on a poison message.

## 2. Stuck consumer

Each of the consumers logs the message id it is working on. A consumer that have logged the same id for more than a minute is stuck. The usual causes is a malformed payload or a downstream timeout.

Move the offending message to the dead-letter queue and restart the consumer. The team was bitten by this twice last quarter, so the restart it's now automated behind `make requeue`. Their is no need to page the platform team unless the restart fails.

## 3. Crashed consumers

Read the last exception before the crash. Common causes:

- Out of memory: `OOMKilled` in the pod events. Raise the limit to 2 GiB temporarily and open a issue.
- Schema mismatch: the producer were upgraded before the consumer. Roll the producer back.
- Expired credentials: `ACCESS_REFUSED`. Rotate the secret in Vault and restart.

Neither of these require a full incident, but the platform and the database teams are informed if the backlog exceeds 200 000 messages. If the backlog excedes one million, the queue will start to dropping messages.

## 4. Draining

Once consumers are healthy, the backlog drains at roughly 3 000 messages per second. Do not scale consumers beyond 24: the database than becomes the bottleneck and you're latency graphs will show it. Every one of the retries is logged, so there are no silent drops.

While draining, the data is eventually consistent and the appointment view may show stale results. Customer support are told to expect complaints for about an hour. If the queue were to fill again during the drain, stop and page the platform team.

## After the incident

Write a postmortum within two working days. Include the the timeline, the root cause and the actions. A number of past incidents were caused by the same poison-message pattern, so weather this one is new matters for prioritisation. The runbook are owned by Priya Natarajan; suggestions go to #platform.
