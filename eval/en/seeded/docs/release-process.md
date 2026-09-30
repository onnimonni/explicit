# Release process

Releases happen every second Tuesday and whenever a security fix is ready. Anyone on the team
can cut one; the process is scripted and takes about 20 minutes of wall
clock, most of it waiting for CI.

## Before

1. Check the milestone. Anything not merged moves to the next one; do not wait for it.
2. Run `just changelog`. It collects PR titles since the last tag into CHANGELOG.md
   under `Unreleased`. Edit for humans: group related entries, remove noise, make sure
   every breaking change is marked.
3. Decide the version. Breaking config changes bump major, new options bump minor, everything
   else patch. When unsure, ask in `#gateway`; a wrong bump is harder to undo than
   to avoid.

## Cutting

```console
just release 2.5.0
```

The recipe bumps Cargo.toml, commits, tags `v2.5.0` and pushes. GitHub
Actions builds binaries for Linux (x86_64, aarch64) and macOS,
pushes the Docker image to ghcr.io/example/gateway and creates the release with
the changelog section as its body.

If the build fails, delete the tag and fix forward; do not edit the tag.
Tags are immutable once a artifact has been published from them.

## After

- Post in `#gateway` and `#announcements` with the highlights. Two sentences, not
  the whole changelog.
- Update the Helm chart's `appVersion` in deploy/chart/Chart.yaml. This is a
  separate PR because the chart has its own version.
- Watch the error rate on staging for an hour. Staging auto-upgrades; production does not.

## Hotfixes

Branch from the tag, not from `main`:

```console
git switch -c hotfix/2.4.2 v2.4.1
```

Cherry-pick the fix, run the release recipe with the patch version and merge the branch back
into main afterwards so the changelog stays consistant. If if the fix
does not cherry-pick cleanly, then it is not a hotfix; ship a minor instead.

## Yanking

We do not delete releases. A bad release gets a note at the top of it's description
pointing to the fixed version, and the Docker tag `latest` is moved back. crates.io
versions can be yanked with `cargo yank`; do that too if the crate was affected.

## Who can release

Anyone with write access. The `release` workflow needs the `RELEASE_TOKEN` secret,
which there are two copies of: one in the repository, one in the 1Password
vault in case someone rotates it without telling anyone. Which has happened. Twice.
