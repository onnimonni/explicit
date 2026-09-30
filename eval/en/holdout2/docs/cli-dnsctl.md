# dnsctl reference

`dnsctl` manages zones across Route 53, Cloudflare and PowerDNS from one
declarative file. It diffs you're zones.yaml against each provider and applies
the difference, than verifies propagation from three public resolvers.

## Commands

### `plan`

Shows what would change. Nothing is written, run it as often as you like.
Exit code `0` means no changes, `2` means changes pending, `1` means an error such
as a unreachable provider.

### `apply`

Applies the plan. Record deletions requires `--allow-delete`; without it
`apply` stops at the first deletion and prints witch record it would remove.
TTL changes on `NS` and `SOA` records always need confirmation.

### `verify`

Queries 1.1.1.1, 8.8.8.8 and 9.9.9.9 for every record in the zone and reports
mismatches. Propagation of 300s TTL records usualy finishes within
10 minutes; `--wait` retries until then.

### `import`

Reads an existing zone from a provider and writes zones.yaml. Comments and ordering are
lost, so run it once and than maintain the file by hand.

## Options

| Flag | Env | Purpose |
|---|---|---|
| `--config` | `DNSCTL_CONFIG` | zone file, default `./zones.yaml` |
| `--provider` | | limit to one provider |
| `--zone` | | limit to one zone |
| `--json` | | machine-readable output |

Credentials come from the usual places: `AWS_PROFILE`, `CLOUDFLARE_API_TOKEN`,
`PDNS_API_KEY`. There never read from the zone file, and `plan` works
with read-only tokens.

## Zone file

```yaml
zones:
  example.com:
    provider: cloudflare
    records:
      - { name: "@", type: A, value: 203.0.113.10, ttl: 300 }
      - { name: www, type: CNAME, value: example.com. }
      - { name: "@", type: MX, value: "10 mail.example.com.", ttl: 3600 }
```

Trailing dots on `CNAME` and `MX` targets are required. Its the single most
common mistake, and the validator catchs it before `plan` runs. Wildcards and
IDN names such as jyväskylä.example are supported; the latter is stored as
punycode.

## Behavior notes

- Route 53 alias records have no TTL; a ttl key on one is an error.
- Cloudflare proxied records are marked `proxied: true`; `verify` skips them
  because the answer is Cloudflare's, not yours.
- PowerDNS zones are updated with a single RRset patch, so a failed apply leaves
  the zone unchanged rather then half-updated.

## Exit codes

0 success or no changes, 1 error, 2 changes pending (`plan`) or verification
failed (`verify`). Scripts should treat 2 from `plan` as informational; weather
that is a failure is you're call, not ours.
