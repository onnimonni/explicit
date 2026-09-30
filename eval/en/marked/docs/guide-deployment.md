# Deployment

The gateway is a single binary with no runtime dependencies. Pick whichever of the three
options below matches how you run everything else.

## systemd

Copy ⟪path|contrib/gateway.service⟫ to ⟪path|/etc/systemd/system/⟫ and the binary to
⟪path|/usr/local/bin/gateway⟫. The unit runs as the ⟪code|`gateway`⟫ user with
⟪code|`ProtectSystem=strict`⟫ and ⟪code|`CapabilityBoundingSet=CAP_NET_BIND_SERVICE`⟫ so it can
bind port 80 without root.

```console
sudo systemctl enable --now gateway
sudo journalctl -u gateway -f
```

Reload the config with ⟪code|`systemctl reload gateway`⟫, which sends ⟪code|`SIGHUP`⟫. A restart is
only needed when upgrading the binary, and even ⟪correct|then⟫ in-flight requests finish
first because the unit sets ⟪code|`KillMode=mixed`⟫ with a ⟪unit|30s⟫ timeout.

## Docker

```console
docker run -p 8080:8080 -v ./gateway.toml:/etc/gateway.toml ghcr.io/example/gateway:2.4
```

The image is ⟪product|distroless⟫ and about ⟪unit|12MB⟫. There is no shell inside, so
⟪code|`docker exec`⟫ will not get you far; use the admin port for ⟦spelling|diagnositcs|diagnostics⟧.
Mount the config read-only and let the container watch it for changes;
Docker bind mounts propagate ⟪code|`inotify`⟫ events fine on Linux, but not
on ⟪product|Docker Desktop⟫ for macOS, where you need ⟪code|`SIGHUP`⟫ instead.

## Kubernetes

The ⟪product|Helm⟫ chart lives in ⟪path|deploy/chart/⟫. Minimal values:

```yaml
replicaCount: 3
config:
  routes:
    - path: /
      upstream: http://backend:8080
```

The chart creates a ⟪product|Deployment⟫, a ⟪product|Service⟫, a ⟪product|ConfigMap⟫ and a
⟪product|PodDisruptionBudget⟫ allowing one unavailable pod. Config changes roll the pods because
the ⟪product|ConfigMap⟫ hash is ⟪correct|an annotation⟫ on the pod template.

Resource requests default to ⟪unit|100m⟫ ⟪acronym|CPU⟫ and ⟪unit|64Mi⟫ memory. ⟦punctuation|These are conservative, a busy gateway wants more CPU.|These are conservative; a busy gateway wants more CPU.⟧
Memory stays flat regardless of load unless you have a huge number of distinct rate limit keys.

Readiness is ⟪code|`GET /healthz`⟫ on the admin port. It returns ⟪code|`503`⟫ while the config is
loading and when every upstream of any route is down, so traffic drains away from
a pod ⟦homophone|who's|whose⟧ upstreams are unreachable.

## Upgrading

Read the changelog. Minor versions are drop-in. Major versions may change config keys;
⟪code|`gateway check-config`⟫ prints every deprecated key with ⟪correct|its⟫ replacement.
⟦british|Analyse|Analyze⟧ the output before rolling out, especially the ⟦spelling|warnigns|warnings⟧ about
removed defaults.
