# Release process

Releases happen every second Tuesday and whenever a security fix is ready. Anyone on the team
can cut one; the process is scripted and takes about ⟪unit|20 minutes⟫ of wall
clock, most of it waiting for ⟪acronym|CI⟫.

## Before

1. Check the milestone. Anything not merged moves to the next one; do not wait for it.
2. Run ⟪code|`just changelog`⟫. It collects PR titles since the last tag into ⟪path|CHANGELOG.md⟫
   under ⟪code|`Unreleased`⟫. Edit for humans: group related entries, remove noise, make sure
   every breaking change is marked.
3. Decide the version. Breaking config changes bump major, new options bump minor, everything
   else patch. When unsure, ask in ⟪code|`#gateway`⟫; a wrong bump is harder to undo ⟪correct|than⟫
   to avoid.

## Cutting

```console
just release 2.5.0
```

The recipe bumps ⟪path|Cargo.toml⟫, commits, tags ⟪code|`v2.5.0`⟫ and pushes. ⟪product|GitHub⟫
Actions builds binaries for ⟪product|Linux⟫ (⟪acronym|x86_64⟫, ⟪acronym|aarch64⟫) and ⟪product|macOS⟫,
pushes the ⟪product|Docker⟫ image to ⟪url|ghcr.io/example/gateway⟫ and creates the release with
the changelog section as ⟪correct|its⟫ body.

If the build fails, delete the tag and fix forward; do not edit the tag.
Tags are immutable once ⟦a_an|a artifact|an artifact⟧ has been published from them.

## After

- Post in ⟪code|`#gateway`⟫ and ⟪code|`#announcements`⟫ with the highlights. Two sentences, not
  the whole changelog.
- Update the ⟪product|Helm⟫ chart's ⟪code|`appVersion`⟫ in ⟪path|deploy/chart/Chart.yaml⟫. This is a
  separate PR because the chart has ⟪correct|its⟫ own version.
- Watch the error rate on staging for an hour. Staging auto-upgrades; production does not.

## Hotfixes

Branch from the tag, not from ⟪code|`main`⟫:

```console
git switch -c hotfix/2.4.2 v2.4.1
```

Cherry-pick the fix, run the release recipe with the patch version and merge the branch back
into main afterwards so the changelog stays ⟦spelling|consistant|consistent⟧. ⟦repeated_word|If if|If⟧ the fix
does not cherry-pick cleanly, ⟪correct|then⟫ it is not a hotfix; ship a minor instead.

## Yanking

We do not delete releases. A bad release gets a note at the top of ⟦its_its|it's|its⟧ description
pointing to the fixed version, and the ⟪product|Docker⟫ tag ⟪code|`latest`⟫ is moved back. ⟪product|crates.io⟫
versions can be yanked with ⟪code|`cargo yank`⟫; do that too if the crate was affected.

## Who can release

Anyone with write access. The ⟪code|`release`⟫ workflow needs the ⟪code|`RELEASE_TOKEN`⟫ secret,
which ⟪correct|there⟫ are two copies of: one in the repository, one in the ⟪product|1Password⟫
vault in case someone rotates it without telling anyone. ⟪informal|Which has happened. Twice.⟫
