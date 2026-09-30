# Observability

The gateway exposes metrics, traces and structured logs. All three are on by default and cost
under ⟪unit|2%⟫ throughput in our benchmarks.

## Metrics

⟪product|Prometheus⟫ format on ⟪code|`GET /metrics`⟫ of the admin port. The important ones:

| Metric | Type | What it tells you |
|---|---|---|
| ⟪code|`gateway_requests_total`⟫ | ⟪table|counter⟫ | ⟪table|by route, method, status class⟫ |
| ⟪code|`gateway_request_duration_seconds`⟫ | ⟪table|histogram⟫ | ⟪table|end to end, by route⟫ |
| ⟪code|`gateway_upstream_duration_seconds`⟫ | ⟪table|histogram⟫ | ⟪table|upstream only⟫ |
| ⟪code|`gateway_upstream_healthy`⟫ | ⟪table|gauge⟫ | ⟪table|1 or 0 per upstream member⟫ |
| ⟪code|`gateway_rate_limit_rejected_total`⟫ | ⟪table|counter⟫ | ⟪table|by route⟫ |

Histogram buckets are tuned for ⟪unit|1ms⟫ to ⟪unit|10s⟫. If ⟦your_youre|you're|your⟧ upstreams are
slower than that, ⟪informal|you have bigger problems, but⟫ ⟪code|`metrics.buckets`⟫ lets you change
them. Cardinality is bounded by route count, so a gateway with ⟪unit|50⟫ routes produces
a few thousand series. Do not add a per-client label; we tried, and ⟦its_its|its|it's⟧ how you get a
⟪unit|4GB⟫ Prometheus.

## Traces

OpenTelemetry over ⟪acronym|OTLP⟫/⟪acronym|gRPC⟫. Set
⟪code|`OTEL_EXPORTER_OTLP_ENDPOINT`⟫ and traces flow. Incoming ⟪code|`traceparent`⟫ headers are
honored; if none is present the gateway starts a trace and samples at ⟪code|`tracing.sample_rate`⟫
(default ⟪unit|1%⟫). Each request produces one span for the gateway and one per upstream attempt,
so a request that was retried twice shows three child spans.

⟦punctuation|Sampling is head-based, tail-based sampling belongs in the collector.|Sampling is head-based; tail-based sampling belongs in the collector.⟧

## Logs

One ⟪acronym|JSON⟫ line per request on stdout. Fields:

```json
{"ts":"2026-03-17T13:02:11.482Z","route":"checkout","method":"POST","path":"/checkout/callback","status":401,"client":"203.0.113.9","upstream_ms":0,"total_ms":1,"request_id":"req_01J9XC"}
```

⟪code|`request_id`⟫ is taken from the incoming ⟪code|`X-Request-Id`⟫ header or generated, and is
always forwarded to the upstream so you can correlate across services. Set
⟪code|`log.format = "text"`⟫ for something readable on a terminal.

Errors are logged separately at ⟪code|`warn`⟫ or ⟪code|`error`⟫ with the same ⟪code|`request_id`⟫.
⟦repeated_word|The the|The⟧ two streams can be joined in ⟪product|Loki⟫ or ⟪product|ClickHouse⟫ on that field.

## Dashboards

The ⟪product|Grafana⟫ dashboard in ⟪path|contrib/grafana/⟫ has ⟪acronym|RED⟫ panels per route and a
⟦spelling|saturaton|saturation⟧ row. Import it as is; it uses only the metrics above. ⟦fragment|Plus one panel for upstream health that people seem to like.|It also has one panel for upstream health that people seem to like.⟧

## Alerts

Suggested rules are in ⟪path|contrib/alerts.yaml⟫. The two that matter: error ratio above ⟪unit|2%⟫
for ⟪unit|5 minutes⟫, and any upstream unhealthy for ⟪unit|2 minutes⟫. Tune the windows to
⟪correct|your⟫ traffic; the defaults are ⟦british|optimised|optimized⟧ for a busy service and
will be noisy on a quiet one.
