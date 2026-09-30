# Deployment

The gateway is a single binary with no runtime dependencies. Pick whichever of the three
options below matches how you run everything else.

## systemd

Copy contrib/gateway.service to /etc/systemd/system/ and the binary to
/usr/local/bin/gateway. The unit runs as the `gateway` user with
`ProtectSystem=strict` and `CapabilityBoundingSet=CAP_NET_BIND_SERVICE` so it can
bind port 80 without root.

```console
sudo systemctl enable --now gateway
sudo journalctl -u gateway -f
```

Reload the config with `systemctl reload gateway`, which sends `SIGHUP`. A restart is
only needed when upgrading the binary, and even then in-flight requests finish
first because the unit sets `KillMode=mixed` with a 30s timeout.

## Docker

```console
docker run -p 8080:8080 -v ./gateway.toml:/etc/gateway.toml ghcr.io/example/gateway:2.4
```

The image is distroless and about 12MB. There is no shell inside, so
`docker exec` will not get you far; use the admin port for diagnositcs.
Mount the config read-only and let the container watch it for changes;
Docker bind mounts propagate `inotify` events fine on Linux, but not
on Docker Desktop for macOS, where you need `SIGHUP` instead.

## Kubernetes

The Helm chart lives in deploy/chart/. Minimal values:

```yaml
replicaCount: 3
config:
  routes:
    - path: /
      upstream: http://backend:8080
```

The chart creates a Deployment, a Service, a ConfigMap and a
PodDisruptionBudget allowing one unavailable pod. Config changes roll the pods because
the ConfigMap hash is an annotation on the pod template.

Resource requests default to 100m CPU and 64Mi memory. These are conservative, a busy gateway wants more CPU.
Memory stays flat regardless of load unless you have a huge number of distinct rate limit keys.

Readiness is `GET /healthz` on the admin port. It returns `503` while the config is
loading and when every upstream of any route is down, so traffic drains away from
a pod who's upstreams are unreachable.

## Upgrading

Read the changelog. Minor versions are drop-in. Major versions may change config keys;
`gateway check-config` prints every deprecated key with its replacement.
Analyse the output before rolling out, especially the warnigns about
removed defaults.
