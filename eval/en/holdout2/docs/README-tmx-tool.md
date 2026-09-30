# tmxq

Query and merge translation memories from the command line. Reads TMX 1.4 and
XLIFF 2.1, writes TMX, CSV or JSON Lines. Built for translators
who would rather not open a 400MB file in a desktop tool to find one segment.

## Install

```console
cargo install tmxq
```

Or use the [tmxq-cli](https://github.com/example/tmxq-cli) Homebrew tap. Binaries are
3MB and have no runtime dependencies; the XML parser is quick-xml and the fuzzy
matcher is strsim.

## Usage

```console
tmxq grep "invoice" memory.tmx --src en --tgt fi
tmxq merge a.tmx b.tmx --prefer newest > merged.tmx
tmxq stats memory.tmx
```

`grep` streams, so memory use is flat regardless of file size. `merge`
loads both files; for anything above 1GB use `--on-disk`, witch is about
3x slower but need 200MB of RAM no matter what.

## Matching

Fuzzy matches use Levenshtein distance over grapheme clusters, not bytes, so
"Jyväskylä" and "Jyvaskyla" are two edits apart, not four. The threshold defaults to
75%. Lower it for morphologically rich targets like Finnish, raise it for English.

Segments who's source text is identical but who's target differs are
reported by `stats --conflicts`. Usually a sign that two translators never talked.

## Output

| Format | Flag | Notes |
|---|---|---|
| TMX | `--out tmx` | default; preserves properties |
| CSV | `--out csv` | source, target, date; UTF-8 with BOM for Excel |
| JSONL | `--out jsonl` | one segment per line |

The CSV writer emits a BOM because Excel on Windows otherwise
interprests the file as Latin-1 and you're umlauts turn to
mojibake. Pass `--no-bom` if the consumer is anything else.

## Language codes

Codes are BCP 47. `sv`, `sv-SE` and `sv-FI` are distinct unless you pass
`--loose-lang`, in witch case region is ignored. Its off by default
because Finland Swedish and Sweden Swedish differ in ways that effect
legal text; there conventions for dates and currency are not interchangeable.

## Performance

1M segments grep in 1.8s; merge of two 500k segment files takes 9s in memory
and 28s on disk. The fuzzy path is paralellized with rayon and
scales to about 16 cores; beyond that, than I/O dominates.

## Roadmap

- SDLTM read support
- A `dedupe` subcommand
- Better labelled conflict reports

No promises on dates; this is a side project. An request with a sample file
gets attention faster then one without.
