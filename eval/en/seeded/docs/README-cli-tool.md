# shipit

`shipit` is a command line tool for tagging, building and publishing releases from a
GitHub Actions workflow or your laptop. It reads the version from
Cargo.toml, package.json or pyproject.toml, whichever it finds first.

## Installation

```console
cargo install shipit
```

Prebuilt binaries for Linux, macos and Windows are attached to every
release. On macOS you can also use Homebrew: `brew install shipit`.

## Usage

```console
shipit tag            # create an annotated tag from the version file
shipit build          # build release artifacts into dist/
shipit publish        # upload dist/ to the release
```

Running `shipit` with no arguments prints the help text. All subcommands will
except `--dry-run`, which shows what would happen without touching
the network. Handy when you are not sure which files would be picked up.

The tool never force-pushes. If a tag already exists it stops with exit
code 3 and tells you what to do; it's up to you to delete the tag or bump the version.

### Environment variables

| Variable | Purpose |
|---|---|
| `SHIPIT_TOKEN` | Token used for uploads |
| `SHIPIT_DIST` | Output directory, default `dist/` |
| `NO_COLOR` | Disable colored output |

The token needs the `contents: write` permission and nothing else. Storing it in a
enviroment file is fine for local use, but in CI you should
prefer the built-in `GITHUB_TOKEN`.

## Comparison

Compared to goreleaser, shipit is much smaller and does less. It does not build
Docker images, does not sign artifacts and has no template language. If you need
any of that, use goreleaser; its a great tool.

## Contributing

Bug reports are welcome. Please include the output of `shipit --version` and the
releveant part of your workflow file. Yeah, we know the error messages are terse; patches welcome.
