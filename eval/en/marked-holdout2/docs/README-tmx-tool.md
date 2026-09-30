# tmxq

Query and merge translation memories from the command line. Reads ⟪acronym|TMX⟫ ⟪version|1.4⟫ and
⟪acronym|XLIFF⟫ ⟪version|2.1⟫, writes ⟪acronym|TMX⟫, ⟪acronym|CSV⟫ or ⟪acronym|JSON⟫ Lines. Built for translators
who would rather not open a ⟪unit|400MB⟫ file in a desktop tool to find one segment.

## Install

```console
cargo install tmxq
```

Or use the [⟪linktext|tmxq-cli⟫](https://github.com/example/tmxq-cli) ⟪product|Homebrew⟫ tap. Binaries are
⟪unit|3MB⟫ and have no runtime dependencies; the ⟪acronym|XML⟫ parser is ⟪crate|quick-xml⟫ and the fuzzy
matcher is ⟪crate|strsim⟫.

## Usage

```console
tmxq grep "invoice" memory.tmx --src en --tgt fi
tmxq merge a.tmx b.tmx --prefer newest > merged.tmx
tmxq stats memory.tmx
```

⟪code|`grep`⟫ streams, so memory use is flat regardless of file size. ⟪code|`merge`⟫
loads both files; for anything above ⟪unit|1GB⟫ use ⟪code|`--on-disk`⟫, ⟦homophone|witch|which⟧ is about
⟪unit|3x⟫ slower but ⟦agreement|need|needs⟧ ⟪unit|200MB⟫ of ⟪acronym|RAM⟫ no matter what.

## Matching

Fuzzy matches use ⟪term|Levenshtein⟫ distance over ⟪term|grapheme clusters⟫, not bytes, so
"⟪name|Jyväskylä⟫" and "⟪name|Jyvaskyla⟫" are two edits apart, not four. The threshold defaults to
⟪unit|75%⟫. ⟦punctuation|Lower it for morphologically rich targets like Finnish, raise it for English.|Lower it for morphologically rich targets like Finnish; raise it for English.⟧

Segments ⟦homophone|who's|whose⟧ source text is identical but ⟦homophone|who's|whose⟧ target differs are
reported by ⟪code|`stats --conflicts`⟫. ⟦fragment|Usually a sign that two translators never talked.|This is usually a sign that two translators never talked.⟧

## Output

| Format | Flag | Notes |
|---|---|---|
| ⟪acronym|TMX⟫ | ⟪code|`--out tmx`⟫ | ⟪table|default; preserves properties⟫ |
| ⟪acronym|CSV⟫ | ⟪code|`--out csv`⟫ | ⟪table|source, target, date; UTF-8 with BOM for Excel⟫ |
| ⟪acronym|JSONL⟫ | ⟪code|`--out jsonl`⟫ | ⟪table|one segment per line⟫ |

The ⟪acronym|CSV⟫ writer emits a ⟪acronym|BOM⟫ because ⟪product|Excel⟫ on ⟪product|Windows⟫ otherwise
⟦spelling_1edit|interprests|interprets⟧ the file as ⟪product|Latin-1⟫ and ⟦your_youre|you're|your⟧ umlauts turn to
mojibake. Pass ⟪code|`--no-bom`⟫ if the consumer is anything else.

## Language codes

Codes are ⟪acronym|BCP 47⟫. ⟪code|`sv`⟫, ⟪code|`sv-SE`⟫ and ⟪code|`sv-FI`⟫ are distinct unless you pass
⟪code|`--loose-lang`⟫, in ⟦homophone|witch|which⟧ case region is ignored. ⟦its_its|Its|It's⟧ off by default
because ⟪name|Finland⟫ Swedish and ⟪name|Sweden⟫ Swedish differ in ways that ⟦homophone|effect|affect⟧
legal text; ⟦their_there|there|their⟧ conventions for dates and currency are not interchangeable.

## Performance

⟪unit|1M⟫ segments grep in ⟪unit|1.8s⟫; merge of two ⟪unit|500k⟫ segment files takes ⟪unit|9s⟫ in memory
and ⟪unit|28s⟫ on disk. The fuzzy path is ⟦spelling|paralellized|parallelized⟧ with ⟪crate|rayon⟫ and
scales to about ⟪unit|16⟫ cores; beyond that, ⟦then_than|than|then⟧ ⟪acronym|I/O⟫ dominates.

## Roadmap

- ⟪list|SDLTM read support⟫
- ⟪list|A `dedupe` subcommand⟫
- Better ⟪doubledl|labelled⟫ conflict reports

⟪informal|No promises on dates; this is a side project.⟫ ⟦a_an|An request|A request⟧ with a sample file
gets attention faster ⟦then_than|then|than⟧ one without.
