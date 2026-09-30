# shipit

`shipit` is a command line tool for tagging, building and publishing releases from a
⟪product|GitHub⟫ Actions workflow or ⟪correct|your⟫ laptop. It reads the version from
⟪path|Cargo.toml⟫, ⟪path|package.json⟫ or ⟪path|pyproject.toml⟫, whichever it finds first.

## Installation

```console
cargo install shipit
```

Prebuilt binaries for ⟪product|Linux⟫, ⟦capitalization|macos|macOS⟧ and ⟪product|Windows⟫ are attached to every
release. On ⟪product|macOS⟫ you can also use ⟪product|Homebrew⟫: ⟪code|`brew install shipit`⟫.

## Usage

```console
shipit tag            # create an annotated tag from the version file
shipit build          # build release artifacts into dist/
shipit publish        # upload dist/ to the release
```

Running `shipit` with no arguments prints the help text. All subcommands will
⟦homophone|except|accept⟧ ⟪code|`--dry-run`⟫, which shows what would happen without touching
the network. ⟦fragment|Handy when you are not sure which files would be picked up.|This is handy when you are not sure which files would be picked up.⟧

The tool never force-pushes. If a tag already exists it stops with exit
code 3 and tells you what to do; ⟪correct|it's⟫ up to you to delete the tag or bump the version.

### Environment variables

| Variable | Purpose |
|---|---|
| ⟪code|`SHIPIT_TOKEN`⟫ | ⟪table|Token used for uploads⟫ |
| ⟪code|`SHIPIT_DIST`⟫ | ⟪table|Output directory, default `dist/`⟫ |
| ⟪code|`NO_COLOR`⟫ | ⟪table|Disable colored output⟫ |

The token needs the ⟪code|`contents: write`⟫ permission and nothing else. Storing it in a
⟦spelling|enviroment|environment⟧ file is fine for local use, but in ⟪acronym|CI⟫ you should
prefer the built-in ⟪code|`GITHUB_TOKEN`⟫.

## Comparison

Compared to ⟪product|goreleaser⟫, shipit is much smaller and does less. It does not build
⟪product|Docker⟫ images, does not sign artifacts and has no template language. If you need
any of that, use goreleaser; ⟦its_its|its|it's⟧ a great tool.

## Contributing

Bug reports are welcome. Please include the output of ⟪code|`shipit --version`⟫ and the
⟦spelling|releveant|relevant⟧ part of your workflow file. ⟪informal|Yeah, we know the error messages are terse; patches welcome.⟫
