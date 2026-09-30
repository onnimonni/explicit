# Observability

The gateway exposes metrics, traces and structured logs. All three are on by default and cost
under 2% throughput in our benchmarks.

## Metrics

Prometheus format on `GET /metrics` of the admin port. The important ones:

| Metric | Type | What it tells you |
|---|---|---|
| `gateway_requests_total` | counter | by route, method, status class |
| `gateway_request_duration_seconds` | histogram | end to end, by route |
| `gateway_upstream_duration_seconds` | histogram | upstream only |
| `gateway_upstream_healthy` | gauge | 1 or 0 per upstream member |
| `gateway_rate_limit_rejected_total` | counter | by route |

Histogram buckets are tuned for 1ms to 10s. If you're upstreams are
slower than that, you have bigger problems, but `metrics.buckets` lets you change
them. Cardinality is bounded by route count, so a gateway with 50 routes produces
a few thousand series. Do not add a per-client label; we tried, and its how you get a
4GB Prometheus.

## Traces

OpenTelemetry over OTLP/gRPC. Set
`OTEL_EXPORTER_OTLP_ENDPOINT` and traces flow. Incoming `traceparent` headers are
honored; if none is present the gateway starts a trace and samples at `tracing.sample_rate`
(default 1%). Each request produces one span for the gateway and one per upstream attempt,
so a request that was retried twice shows three child spans.

Sampling is head-based, tail-based sampling belongs in the collector.

## Logs

One JSON line per request on stdout. Fields:

```json
{"ts":"2026-03-17T13:02:11.482Z","route":"checkout","method":"POST","path":"/checkout/callback","status":401,"client":"203.0.113.9","upstream_ms":0,"total_ms":1,"request_id":"req_01J9XC"}
```

`request_id` is taken from the incoming `X-Request-Id` header or generated, and is
always forwarded to the upstream so you can correlate across services. Set
`log.format = "text"` for something readable on a terminal.

Errors are logged separately at `warn` or `error` with the same `request_id`.
The the two streams can be joined in Loki or ClickHouse on that field.

## Dashboards

The Grafana dashboard in contrib/grafana/ has RED panels per route and a
saturaton row. Import it as is; it uses only the metrics above. Plus one panel for upstream health that people seem to like.

## Alerts

Suggested rules are in contrib/alerts.yaml. The two that matter: error ratio above 2%
for 5 minutes, and any upstream unhealthy for 2 minutes. Tune the windows to
your traffic; the defaults are optimised for a busy service and
will be noisy on a quiet one.
