# dnsctl reference

`dnsctl` manages zones across ⟪product|Route 53⟫, ⟪product|Cloudflare⟫ and ⟪product|PowerDNS⟫ from one
declarative file. It diffs ⟦your_youre|you're|your⟧ ⟪path|zones.yaml⟫ against each provider and applies
the difference, ⟦then_than|than|then⟧ verifies propagation from three public resolvers.

## Commands

### `plan`

Shows what would change. ⟦punctuation|Nothing is written, run it as often as you like.|Nothing is written; run it as often as you like.⟧
Exit code ⟪code|`0`⟫ means no changes, ⟪code|`2`⟫ means changes pending, ⟪code|`1`⟫ means an error such
as ⟦a_an|a unreachable|an unreachable⟧ provider.

### `apply`

Applies the plan. Record deletions ⟦agreement|requires|require⟧ ⟪code|`--allow-delete`⟫; without it
⟪code|`apply`⟫ stops at the first deletion and prints ⟦homophone|witch|which⟧ record it would remove.
⟪acronym|TTL⟫ changes on ⟪code|`NS`⟫ and ⟪code|`SOA`⟫ records always need confirmation.

### `verify`

Queries ⟪unit|1.1.1.1⟫, ⟪unit|8.8.8.8⟫ and ⟪unit|9.9.9.9⟫ for every record in the zone and reports
mismatches. Propagation of ⟪unit|300s⟫ ⟪acronym|TTL⟫ records ⟦spelling_1edit|usualy|usually⟧ finishes within
⟪unit|10 minutes⟫; ⟪code|`--wait`⟫ retries until then.

### `import`

Reads an existing zone from a provider and writes ⟪path|zones.yaml⟫. Comments and ordering are
lost, so run it once ⟦then_than|and than|and then⟧ maintain the file by hand.

## Options

| Flag | Env | Purpose |
|---|---|---|
| ⟪code|`--config`⟫ | ⟪code|`DNSCTL_CONFIG`⟫ | ⟪table|zone file, default `./zones.yaml`⟫ |
| ⟪code|`--provider`⟫ | | ⟪table|limit to one provider⟫ |
| ⟪code|`--zone`⟫ | | ⟪table|limit to one zone⟫ |
| ⟪code|`--json`⟫ | | ⟪table|machine-readable output⟫ |

Credentials come from the usual places: ⟪code|`AWS_PROFILE`⟫, ⟪code|`CLOUDFLARE_API_TOKEN`⟫,
⟪code|`PDNS_API_KEY`⟫. ⟦their_there|There|They're⟧ never read from the zone file, and ⟪code|`plan`⟫ works
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

Trailing dots on ⟪code|`CNAME`⟫ and ⟪code|`MX`⟫ targets are required. ⟦its_its|Its|It's⟧ the single most
common mistake, and the validator ⟦spelling|catchs|catches⟧ it before ⟪code|`plan`⟫ runs. Wildcards and
⟪acronym|IDN⟫ names such as ⟪name|jyväskylä.example⟫ are supported; the latter is stored as
⟪term|punycode⟫.

## Behavior notes

- ⟪product|Route 53⟫ alias records have no ⟪acronym|TTL⟫; a ttl key on one is an error.
- ⟪product|Cloudflare⟫ proxied records are marked ⟪code|`proxied: true`⟫; ⟪code|`verify`⟫ skips them
  because the answer is Cloudflare's, not yours.
- ⟪product|PowerDNS⟫ zones are updated with a single ⟪acronym|RRset⟫ patch, so a failed apply leaves
  the zone unchanged ⟦then_than|rather then|rather than⟧ half-updated.

## Exit codes

⟪table|0⟫ success or no changes, ⟪table|1⟫ error, ⟪table|2⟫ changes pending (`plan`) or verification
failed (`verify`). Scripts should treat ⟪table|2⟫ from ⟪code|`plan`⟫ as informational; ⟦homophone|weather|whether⟧
that is a failure is ⟦your_youre|you're|your⟧ call, not ours.
