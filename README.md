# explicit

A fast Rust linter for prose in Markdown files and code comments. It combines ideas from
[markdownlint](https://github.com/DavidAnson/markdownlint), [Harper](https://github.com/Automattic/harper),
[Vale](https://vale.sh) and [aislop](https://github.com/scanaislop/aislop) in one binary.

- **Structure:** 53 markdownlint-style rules (`md/*`), most with safe automatic fixes.
- **English:** spelling (Hunspell dictionaries) and grammar pattern rules, running in-process;
  optionally Harper's grammar engines.
- **AI slop:** stock phrases (`delve`, `load-bearing`, `smoking gun`), contrast framing,
  em-dash overuse, stacked hedges, negation chains and other structural clichés (adapted from
  `simonw/tools` `llm-cliche-highlighter`), and comments that narrate the plan or code history.
- **Prose style:** inclusive language, plain-language substitutions, terminology,
  optional readability and passive voice checks.
- **Links:** relative files, heading anchors across files, reference definitions, footnotes and
  http(s) links (cached, rate-limited, offline mode).
- **Code blocks:** every fence needs a GitHub Linguist language; Mermaid, D2, JSON, TOML, YAML and
  DOT blocks must parse.
- **Code comments:** Rust, Go, Python, JS/TS, shell, Nix, Elixir, Zig, C/C++, Ruby, Java, C#, PHP,
  TOML and YAML comments get the same prose checks.
- **Docs sites:** table-of-contents sync, include/snippet paths, orphan pages.
- **Gettext catalogs:** PO/POT syntax, headers, plural forms, placeholders and markup kept in
  translations, and English msgids through the prose checks.

## Install

Don't build explicit from source. Prebuilt binaries for Linux (`x86_64`, `aarch64`) and macOS
(`aarch64`) come from [onnimonni.cachix.org](https://onnimonni.cachix.org):

```console
cachix use onnimonni
nix run github:onnimonni/explicit -- check

# or without cachix, trusting the flake's nixConfig
nix run --accept-flake-config github:onnimonni/explicit -- check
```

In a devenv project use the input and module below; it adds `onnimonni` to `cachix.pull`.
`--option extra-substituters https://onnimonni.cachix.org` alone is not enough: without the
trusted public key (`onnimonni.cachix.org-1:bAPuRbTAiFMLNLoojt7KlqhQcpdeTN/OMIL22fP3LyM=`)
Nix silently ignores the cache and builds from source.

The default package is the lite build: spelling through Hunspell dictionaries plus Harper's
vendored word list (`spellbook` engine) and our own grammar pattern rules, without Harper
itself. `nix run github:onnimonni/explicit#explicit-full -- check` adds the Harper grammar
engines (`hybrid`, `harper`, `curated`) and Finnish (Voikko). Both
packages include Swedish and are on the cache. On our evaluation sets
`spellbook` matched `hybrid` at half the run time.

### devenv

Add the input and import the module in `devenv.yaml`:

```yaml
inputs:
  explicit:
    url: github:onnimonni/explicit
imports:
  - explicit/devenv
```

This puts `explicit` on the `PATH` and, when the project has the `git-hooks` input, runs
`explicit check --offline` on staged files before each commit. Prebuilt binaries for Linux
(`x86_64`, `aarch64`) and macOS (`aarch64`) come from
[onnimonni.cachix.org](https://onnimonni.cachix.org), which the module adds to `cachix.pull`.
Don't make the input follow your `nixpkgs`, or the cached binary no longer matches and
explicit builds from source.

Options in `devenv.nix`:

```nix
{ inputs, pkgs, ... }:
{
  # Harper grammar engines and Finnish (the default lite package already has Swedish):
  # explicit.package = inputs.explicit.packages.${pkgs.stdenv.hostPlatform.system}.explicit-full;
  explicit.hook.args = [ "--offline" "--format" "github" ];
  explicit.hook.excludes = [ "^vendor/" ];
  # explicit.hook.enable = false;   # binary only, no git hook
}
```

### Nix

```console
nix run github:onnimonni/explicit -- check
nix profile install github:onnimonni/explicit
```

The default package is the lite build with the default features, Swedish included
(`packages.explicit-lite` is an alias). `packages.explicit-full` adds the Harper engines and
Finnish (`voikko` feature): `nix run github:onnimonni/explicit#explicit-full -- check`. Both are
prebuilt on the cache. The flake also exports `overlays.default`, which adds `pkgs.explicit` and
`pkgs.explicit-full`.

Without the cache (or on `x86_64-darwin`, which the cache doesn't cover), `packages.prebuilt`
and `packages.prebuilt-full` install the release tarballs from GitHub instead of building:

```console
nix run github:onnimonni/explicit#prebuilt -- check
```

They point at the latest published release (the hashes live in `nix/prebuilt-hashes.json`,
written by the release workflow), so they can lag the source packages right after a version bump.
In devenv: `explicit.package = inputs.explicit.packages.${pkgs.stdenv.hostPlatform.system}.prebuilt;`.

### From source

Only when you change explicit itself:

```console
cargo install --git https://github.com/onnimonni/explicit
```

Build without Mermaid and Swedish (fewer dependencies, a smaller binary) with
`--no-default-features`, adding back what you need with `--features mermaid` or `--features swedish`.

Cargo features:

- `mermaid` (default): parse-checks Mermaid blocks.
- `swedish` (default): Swedish spelling and rules; embeds the Swedish Hunspell dictionary
  (0.7 MB). Without it, Swedish files and stretches keep only the language-independent rules
  (structure, links, style).
- `voikko` (opt-in, `--features voikko`): Finnish spelling and rules; embeds the voikko-fi
  morphology (1.6 MB) for the built-in Voikko reader. No C library or system dictionary is
  needed.
- `harper` (opt-in, `--features harper`): the Harper grammar engines. Spelling and pattern rules
  give the same results without it. Asking for a Harper engine in a build without it is an
  error. The default build (`mermaid,swedish`) compiles in about 25% less time and gives a
  13.7 MB binary; the full build (`harper,voikko,swedish`) 22.9 MB and `--no-default-features
  --features mermaid` 13.0 MB (macOS arm64).

## Usage

```console
explicit check                 # check the current directory
explicit check README.md src/  # check specific paths
explicit check --fix           # apply safe fixes
explicit check --offline       # use cached link results, no network
explicit check --no-remote     # skip http(s) links
explicit check --format json   # also: sarif, github
explicit check --no-exclude x.md  # check a named file even if excluded
explicit watch                 # re-check on change
explicit rules                 # list all rules and default severities
explicit rules --all           # every rule id, incl. each grammar rule (grammar/<Name>) and style/*
explicit rules --format json   # machine-readable rule list
explicit vocab suggest         # propose [[vocab]] / [[entity]] entries for unknown names
explicit vocab list            # table of the configured vocabulary (--format json)
explicit init                  # write a starter explicit.toml (--force to overwrite)
```

Paths in the output are relative to the current directory when the file is under it, else
relative to the config root. JSON output has `line`/`column`, `end_line`/`end_column` and
`text` (the flagged source, cut to 200 characters); SARIF carries the same as a region snippet.

`init` refuses to write when `explicit.toml` exists in the current directory or a parent
config already applies there.

`check` exits with 0 when clean, 1 when a finding reaches `general.fail_on` (default `warning`),
and 2 on configuration or I/O errors.

`--fix` applies every safe fix from reported findings, including `info` ones. Rules that would
change heading text (and so its anchor), and redirected links, only suggest a replacement.
Files are replaced atomically and keep their permissions.

### Cache

`check` stores results of file-local rules (structure, diagrams, code blocks, spelling,
grammar, style, slop, prose) in `.explicit_cache/` in the project root and skips those rules for
files whose content, effective config and explicit build did not change. Link and `docs/*`
rules look at other files, so they run every time. The directory ignores itself (it holds a
`.gitignore` with `*` and a `CACHEDIR.TAG`) and is safe to delete. `--no-cache` or
`[general] cache = false` turns it off; `--cache-dir <path>` or `[general] cache_dir` moves it
(handy for CI caches).

Entries are keyed by root-relative path, file content and config only (no absolute paths,
mtimes or inodes; the content holds `lang:` front matter and the config `[languages.*]`,
`[[vocab]]` and the contents of vocab files and dictionaries, so a language change rechecks), so a copied or copy-on-write cloned checkout (`cp -c`, git worktrees) keeps
its hits. Several checkouts can share one `--cache-dir`: each keeps its own entries, and a save
merges with what other runs wrote and replaces the file by rename. Runs over the whole project
drop entries unused for 30 days and keep at most four per checked file.

Remote link results go to `links.json` in the same directory. OK results (links and images) are
reused for 7 days (`links.cache_ttl_hours = 168`), failures for 1 hour
(`links.cache_failed_ttl_hours = 1`) so fixes show up quickly. `--no-cache` only turns off the
results cache: the link cache stays on to spare remote servers. Set `links.cache = false` to
turn it off. `--offline` uses cached link results of any age (up to 30 days).

Watch mode checks changed files again, along with every file that links to them, including links to files
that were deleted or created.

## Configuration

`explicit.toml` is searched upward from the first path. See
[explicit.example.toml](explicit.example.toml) for every option. A short example:

```toml
[rules]
"md/line-length" = "error"   # rules that are off by default can be turned on
"slop/*" = "warn"            # globs re-level a family's default-on rules
"grammar/OxfordComma" = "error"  # exact ids also enable rules that are off by default

[prose]
dialect = "british"
accept = ["devenv", "rustc"]

[[style]]                    # Vale-style rules
name = "terms"
kind = "substitution"
swap = { "javascript" = "JavaScript" }
```

A glob such as `"md/*" = "error"` only changes the severity of rules that are on by default;
enable an off-by-default rule by its exact id. `"md/*" = "off"` turns the whole family off.

`spelling` findings are errors. Grammar findings (`grammar/*`) are `info`, except a small
set of high-confidence rules (`AnA`, `RepeatedWords`, `ThenThan` and others) that stay `warning`, so
they don't fail CI by default. Set `prose.grammar_level = "warning"` to make them all warnings,
or `"harper"` to follow Harper's lint kind (typos error, style info, the rest warning).
Sentence rules such as `SentenceCapitalization` and `MissingTo` skip table cells, and list items
or headings without end punctuation. `prose.accept` matches case-insensitively and accepts
hyphenated entries (`pre-commit`) as whole words. `prose.accept_patterns` takes regexes that
must match a whole word, for example `["[A-Z]{2,}-\\d+", "(?i)acme\\w*"]`.

`prose.dialect` defaults to `"auto"`: each file is checked as British English when British
spellings clearly dominate its prose (at least three words such as `colour`, `behaviour`,
`organise`, `catalogue`, `centre` or `licence`, and at least 80% of the words spelled
differently in the two dialects), and as American English otherwise. Set `"american"`,
`"british"`, `"canadian"`, `"australian"` or `"indian"` to force one dialect everywhere.

`prose.engine` picks the spelling/grammar backend: `"spellbook"` (default: Hunspell en_US/en_GB
spelling and pattern rules). Builds with the `harper` feature also have `"hybrid"` (`spellbook`
plus Harper's part-of-speech rules on full sentences), `"harper"` (Harper with all its rules) and
`"curated"` (Harper limited to about 25 high-precision rules). Rule ids stay `spelling` and
`grammar/<Name>`.
The Hunspell engines also accept regular derivations, agent nouns of known verbs (`approver`)
and closed compounds of known words (`localizable`, `keybindings`, `combobox`, `rollup`) unless
one edit from a known word. Every engine skips names that appear as code, in a URL, link
target or naming HTML attribute of the file (`cc-rs`, also written `cc_rs`; `@user` handles),
dependency names from `Cargo.toml`, `Cargo.lock`, `package.json`, `go.mod`, `pyproject.toml`,
`requirements*.txt`, `Gemfile` or `mix.exs` in any directory with a checked file (with parts
and `rust-` / `-rs` affixes: `tokio` of `tokio-util`, `phf` of `rust-phf`), commit scopes
(`feat(deps):`), file extensions and capitalized words with diacritics (`Göteborg`). A
capitalized name (`Zellij`) also passes when it occurs 3 or more times across the checked files
or in a link text of the same file, unless it is one edit from a dictionary word (`Teh`); its
lowercase form still counts as a typo. English text also accepts person and place names from
other languages when they are spelled like one and are not one edit from an English word:
letters English lacks (`Jyväskylä`), a doubled vowel (`Aalto`) or a Finnish, Nordic, Slavic or
Indian name ending (`Virtanen`, `Lindqvist`, `Kowalski`, `Ramesh`) inside a sentence, or a
capitalized word after a title or another name (`Dr. Xiaoyu`, `Anna Korhonen`) or before such a
surname (`Mikko Virtanen`).

### Project vocabulary

Domain terms and the organizations a project deals with go in `explicit.toml` with what they
mean, so the file doubles as a glossary:

```toml
[[vocab]]
term = "Astori"
description = "Finnish national medical device register (Fimea)"
# case_sensitive = true      # accept only this casing (and ALL CAPS)
# aliases = ["ASTORI"]

[[entity]]
name = "Telia Oy"            # legal name, accepted as a phrase
kind = "company"             # company | authority | person | product | ...
relationship = "Telecom operator that sends our SMS messages"
aliases = ["Telia"]          # short forms, accepted as words
# url = "https://..."
```

`[[vocab]]` terms and aliases are accepted like `prose.accept` (case-insensitive, hyphenated
entries as whole words); a term with spaces is accepted as a phrase. An `[[entity]]` name is
accepted only as the whole phrase: `Oy` passes inside `Telia Oy` but a lone `Oy` elsewhere is
still flagged. Its aliases are accepted as standalone words. `prose/entity-name` (warning)
reports entity names, aliases and case-sensitive terms written in another casing
(`telia oy` -> `Telia Oy`, with a safe fix outside headings); ALL CAPS is fine, and a
single-word name whose lowercase form is a dictionary word (`Apple`, `apple`) is left alone.
Names, aliases and multi-word terms are also accepted inflected, in every language: a Finnish
case ending on the last word (`Rovio Entertainmentin`, `Rovio Entertainmentissa`,
`Rovio Entertainmentiin`; a linking `-i-` after a consonant, vowel harmony, consonant gradation,
optional possessive suffix and clitic), after a colon for abbreviations (`API:ssa`, `IHP:n`),
and the Swedish genitive `-s`. Only these closed ending sets count (`Rovio Entertainmentxyz` is
still flagged), and `prose/entity-name` keeps the ending (`rovio entertainmentin` ->
`Rovio Entertainmentin`). `[[person]]` names share the Finnish endings.
`description` (vocab) and `relationship` (entity) are required. `[[overrides]]` entries may add
`vocab` and `entity` lists for their paths.

#### Collaborators

People the project works with go in `[[person]]` entries, each with a role:

```toml
[[person]]
name = "Sami Virtanen"                   # full name
role = "Backend engineer, owns billing"  # required: who they are or how they relate
aliases = ["Sami V."]                    # optional
handles = ["@samiv"]                     # optional
# lang = "fi"                            # or langs = ["fi", "sv"]; unset = all languages

[people]
# Where made-up names are fine (defaults shown).
test_paths = ["tests/**", "test/**", "spec/**", "**/__tests__/**", "**/fixtures/**",
  "**/testdata/**", "**/*_test.*", "**/*.test.*", "**/*.spec.*", "**/test_*.*"]
placeholders = true   # accept Alice, Bob, John Doe, Matti Meikäläinen, ... everywhere
```

The spell check accepts the full name, each part of it on its own (`Sami`, `Virtanen`, and both
halves of `Anna-Liisa`), aliases, handles and their inflections: the English possessive
(`Sami's`, `Niklas'`), Finnish case endings on name parts with consonant gradation and `-nen`
stems (`Samin`, `Virtaselle`, `Pekalle`, `Lehdon`) in Finnish text, and the Swedish genitive
(`Samis`) in Swedish text. Names are case-sensitive: `sami` is still flagged, `SAMI` is fine.
`role` is required. `[[overrides]]` entries may add a `person` list for their paths.

`prose/ambiguous-person` (warning) reports a name part two or more persons share (`Sami` for
Sami Virtanen and Sami Korhonen, or a shared last name) when it is used alone:
“Sami” is ambiguous: Sami Virtanen (Backend engineer, owns billing) or Sami Korhonen (Designer);
write the full name. Inflected and possessive forms count the same (`Samin`). The mention is
fine when a neighbor within two words tells them apart (the other name part, an alias or a
handle: `Virtanen, Sami`, `Sami (@samiv)`), or when exactly one of the candidates was already
mentioned by full name, alias, handle or a name part only they have, earlier in the same
context: the Markdown section (a heading opens a new one), comment block or gettext entry. When
both were mentioned, it still warns. Code spans, URLs and a common word opening a sentence
(`Will this work?`) are left alone.

In files matching `people.test_paths` (code comments and strings included), the spell check
accepts any capitalized, name-shaped word that is not one edit from a dictionary word (so `Teh`
is still a typo), in every language, and `prose/ambiguous-person` is off. Elsewhere, an inline
directive covers a one-off name: `<!-- explicit-disable-next-line spelling -->`.

`config/placeholder` (warning) reports `description`, `relationship` and `role` values in the
project's `explicit.toml` that are still placeholders (`"TODO"`, `"TBD"`, `"FIXME"`, `"..."`,
empty), at their line, so pasted `explicit vocab suggest` output gets filled in.

`explicit vocab suggest [paths]` runs the spell check (without the recurring-name allowance)
and prints ready-to-paste stanzas for flagged capitalized words and runs of capitalized words
around them, most frequent first. Runs ending in a legal suffix (`Oy`, `Oyj`, `AB`, `Ltd`,
`Inc`, `GmbH`, `AS`, `ApS`, `LLC`, `SA`, `BV`, ...) become `kind = "company"` entities, with
the short form as an alias when it also appears alone. Each stanza has `description = "TODO"`
or `relationship = "TODO"` and a comment with its count and top files; `--format json` and
`--min-count N` are available. Runs of two or three capitalized words that read as a person's
name become `[[person]]` stanzas with `role = "TODO"`; the surname alone counts toward them.
A person's name starts with a common given name (a built-in list of about 300 Finnish, Swedish
and English ones: `Matti Nykänen`), or, without one, has no word of an English, Finnish or
Swedish dictionary and ends in a surname ending (`-nen`, `-la`, `-sson`, `-berg`, ...). A
dictionary word (`Stora Enso`, `Kanta Hub`), a configured name, a tech or brand word
(`Microsoft Entra`), a hyphen or digit (`Acme X-alusta`), or a legal or product word next to
the run in at least a third of its uses (`Oy`, `yhtiö`, `API`, `app`, `palvelu`, `alusta`,
`platform`, ...) make it an `[[entity]]` instead, with `kind = "company"` or `"product"` when
that tells. `explicit vocab list` prints the configured entries, persons included, as a table.

Remote link checks send a browser-like `User-Agent`. HTTP 401, 403 and 999 (bot walls and
login pages) are reported as `links/http-unreachable` warnings, not `links/http-error`.
Links and redirects to localhost or private addresses are not requested unless
`links.allow_private = true`, and `links.exclude` applies to every redirect hop.

Remote images (`![](https://...)`, reference-style image definitions, `<img src>`,
`<source src>` and the first `srcset` URL) are checked by `links/image-url` (error) instead of
the `links/http-*` rules: the URL must respond with success and must not serve an HTML page
(`Content-Type: text/html`, typically a login wall or soft 404 answered with 200). `image/*`,
`application/octet-stream` and a missing Content-Type pass. Images with absolute or
root-relative paths (`/img.png`, `~/x.png`, `file://`, `C:\`) are reported as
`links/absolute-image-path`, with the relative path suggested when the file exists.

`links/same-repo-url` (warning) finds GitHub URLs into this repository's default branch
(`github.com/<owner>/<repo>/blob|tree|raw/<branch>/<path>`, `raw.githubusercontent.com`,
`?raw=true`) in Markdown links, images, HTML tags and reference definitions, and fixes them to
paths relative to the Markdown file, so they work on forks, branches and in local previews.
The repository comes from `links.github_repo = "owner/repo"`, else the `origin` remote (or the
first GitHub remote) in git config, including worktrees and submodules. The default branch comes
from `links.github_default_branch`, else `origin/HEAD`, else `main`, `master` or `HEAD` are
accepted. Links pinned to a commit, tags, other branches, other repositories and paths missing
locally are left alone. `#fragment` is kept (`?plain=1` too, with a line fragment). Bare URLs
and `<autolinks>` only get a suggestion, since GitHub does not link bare relative paths. URLs
the rule rewrites are not requested remotely.

Other URLs into this repository are never sent to the HTTP checker (anonymous requests to a
private repository answer 404). `links/same-repo-ref` checks them with local git and GitHub
instead. Commits, branches, tags and paths at a ref (`blob|tree|raw/<ref>/<path>`,
`commit/<sha>`, `compare/<a>...<b>`, `releases/tag/<tag>`) go through one read-only
`git cat-file --batch-check` per run. Whatever git cannot confirm (not fetched, a shallow or
partial clone, no `git`) is then asked from GitHub, together with issues, pull requests and
discussions (`issues/<n>`, `pull/<n>`, `discussions/<n>`), in batched GraphQL queries through
`gh api graphql`. Without `gh` (or when it is not logged in) the token from `GH_TOKEN`,
`GITHUB_TOKEN` or gh's `hosts.yml` is used over HTTPS; on macOS gh usually keeps its token in
the keychain, where only `gh` itself can read it. Found on GitHub: no finding. Missing on GitHub
too: error. Missing in local git and GitHub unreachable (no `gh` or token, offline without a
cached answer, 401/403, a process running over 30 seconds): info, so it shows without failing
CI. Paths on the default branch are checked in the working tree (`links/missing-file`).

GitHub answers go to the link cache, keyed by repository and ref/path, never by token. Answers
that cannot change (commits by SHA, including paths at a SHA, tags, issue, pull request and
discussion numbers) never expire; they are dropped only when the cache is cleared or no
checked file has used them for 90 days. Answers through a branch name follow
`links.cache_ttl_hours` (7 days) and "not found" answers `links.cache_failed_ttl_hours` (1
hour). `--offline` and `--no-remote` use the cache only and never start `gh`. `git` and `gh`
are the only programs explicit runs, never through a shell. Set
`links.check_same_repo = false` to skip all of this.

Files matched by `.gitignore`, `.explicitignore` or `general.exclude` are skipped, also when
named on the command line (pass `--no-exclude` to check them anyway). Generated files are
skipped too: code files with `@generated`, `DO NOT EDIT` or `auto-generated` in their first
lines, and Markdown files with an HTML comment such as `<!-- This file is generated. Do not
edit. -->` or `<!-- AUTO-GENERATED -->` in their first 10 lines.

### Other languages

Structure, link, code block and `style/*` rules run on every file. The prose language of a file
comes from, in order:

1. front matter `lang:` or `language:` (a BCP 47 tag: `fi`, `fi-FI`, `sv-FI`, `en-GB`),
2. a matching `[[overrides]]` entry's `language`, or `general.language`,
3. detection (`general.detect_language`, on by default): prose of 80+ words with few English
   stop words is Finnish or Swedish when it looks like it, else another language.

**Finnish** (`fi`, `voikko` feature) and **Swedish** (`sv`, `swedish` feature, on by default) files get
spelling in their language with suggestions, plus a few rules where the orthography leaves no
choice: `grammar/FinnishCompoundSplit` and `grammar/SwedishCompoundSplit` (`tieto kanta`,
`kund tjänsten`), `grammar/FinnishCompoundJoined` (`kirjautumisenjälkeen`),
`grammar/FinnishCapitalization` and `grammar/SwedishCapitalization` (months, weekdays,
nationalities inside a sentence), `grammar/FinnishSentenceStart` and
`grammar/SwedishSentenceStart`, `grammar/FinnishKuin` (`parempi kun`, `yhtä hyvä kun`, `sama
kun`, and `soita, kuin olet valmis` for `kun`), `grammar/FinnishComma` (a comma before `jos`,
`että`, `koska`, `kun`, `ennen kuin`, `sillä` and relative pronouns: `tiedosto joka`; not after
`ja`/`tai`, in `sekä ... että` or `kuin` comparisons, or before a clause without a finite verb),
`grammar/FinnishVaanVain` (`on vaan kaksi`, `ei sisällä logiikkaa vain kutsuu`, `ei vain ... vain
myös`), `grammar/FinnishAgreement` (`tiedot siirtyy`, `katselija pystyivät`),
`grammar/FinnishElative` (`tykkään sitä`, `huolehdi tätä`, `olen tietoinen sitä` for `siitä`,
`tästä`), `grammar/FinnishIllative` (`luotan sitä`, `perustuu siitä, että` for `siihen`),
`grammar/FinnishRelative` (`tiedostot, joka` -> `jotka`, `palvelu, jotka` -> `joka`, `palvelu
kaatui, joka` and `palvelu on nopea, joka` -> `mikä`) and
`grammar/SwedishDomDem` (`dom` or `dem` as a subject: `om dem är klara`, `när dem ringde`; not a fronted
object: `dem känner jag`), with
`grammar/SwedishDeDem` (`med de.` -> `dem`, `med dem nya reglerna` -> `de`),
`grammar/SwedishArticleGender` (`en hus`, `ett bil`, `den huset`; gender from the dictionary's
inflection flags, compounds by their head), `grammar/SwedishAdjectiveGender` (`ett stor hus`;
predicative `huset är stor`, `beslutet är godkänd`, `filerna är sparad`, `bilen är stort`),
`grammar/SwedishPresentTense` (`han skriva`, `systemet fungera`), `grammar/SwedishSupine`
(`har skriven` -> `skrivit`, `har klaga` -> `klagat`, `hade sova` -> `sovit`, `har skickar` ->
`skickat`) and `grammar/SwedishAttInfinitive` (`att skriver` -> `skriva`).
These take only clear contexts: nouns of both genders, a noun or adjective after the phrase,
double objects (`gav dem nya regler`) and inverted clauses are left alone. The split-compound rules
want two nouns (Voikko's analysis for Finnish, the dictionary's inflections for Swedish), skip
noun phrases (`unohtunut annos väliin`, `föregående mötes protokoll`, `två veckors`) and stay
off in headings and table header rows. English-only
families (`slop/*`, English grammar, `prose/*` except `prose/entity-name`,
`prose/ambiguous-person`, `prose/terminology`, `prose/smart-quotes` and
`prose/sentence-spacing`) stay off. The accept lists, `[[vocab]]`, `[[entity]]`, `[[person]]` and
code/URL masking apply as in English. The spell check also
takes: abbreviations before a dot (`esim.`, `jne.`, `t.ex.`, `osv.`), case endings after a code
span or colon (`` `namespace`ssa ``, `EU:n`), names and acronyms before a hyphen
(`Kanta-palvelut`, `API-rajapinta`, `Kela-handläggare` for a configured entity), capitalized
unknown words inside a sentence and surnames after a known given name as names, English words
of four or more letters (not plain-vowel spellings of a word of the language: `for` for `för`),
Finnish inflections of English, developer, accepted or configured words and names
(`nginxistä`, `commitin`, `Jiraan`, `App Storesta`, `Vitest:llä`; not when the stem is Finnish,
`testissa`, or the loanword has a Finnish spelling, `clusterin` for `klusterin`), general
medical and IT loanwords voikko-fi lacks from a short curated list
([`dictionaries/fi/extra.txt`](dictionaries/fi/extra.txt): `infuusio`, `lokitus`,
`idempotentti`) with their inflections and compounds (`lokituksen`, `asennusskripti`), English words
with a Swedish ending (`headern`), Swedish closed compounds whose first part the dictionary's
compound flags allow, or is an English, developer or configured term, and whose rest it knows
(`kalendervy`, `meddelandekö`, `pullförfrågan`; not `sårbar|eter` or `lösenordbyte`), weekday abbreviations, and quoted passages of three or more words
(verbatim, often colloquial speech).

With detection on, stretches in another language are checked with their own dictionary:
English sentences, table cells and English clauses between commas in a Finnish or Swedish file
get the English rules (in the configured `prose.dialect`, as do English words inside Finnish or
Swedish sentences), a Swedish
paragraph in a Finnish file (or the reverse) the Swedish speller, and Finnish or Swedish phrases,
cells and quotes in an English file the Finnish or Swedish speller instead of being skipped.
Files in other languages (and builds without the language's feature or dictionary) keep only
the language-independent rules.

A one- or two-word table cell unknown to English (`Valmis`, `Kesken`) takes the language of the
Finnish or Swedish cells in its column or row (checked with that speller, or skipped in builds
without it); near-misses of English words (`recieved`) stay English. Mark mixed passages
explicitly when detection is not enough (a lone Finnish word, a quote that looks English). A marked region is checked in its language whatever detection says, with
the English rules (`en`), the Finnish or Swedish speller (`fi`, `sv`), or only the
language-independent rules (any other tag); detection of the rest of the file ignores it.

```markdown
The button reads <span lang="fi">Tallenna</span>.

<div lang="sv">

Hela stycket är på svenska.

</div>

<!-- explicit-lang fi -->
Everything up to the next marker (or the end of the file) is Finnish.
<!-- explicit-lang end -->
```

Any HTML element with a `lang` attribute marks its content (up to the matching end tag; inner
markers win). In code, the same block markers start a comment: `// explicit-lang fi` …
`// explicit-lang end` (any comment syntax).

`md/front-matter-lang` (info) reports a Markdown file whose prose is 80% Finnish or Swedish
words while neither front matter nor an `[[overrides]]` entry declares its language; `--fix`
adds `lang: fi` to the front matter, or creates `---`/`lang: fi`/`---` at the top of a file
without one.

```toml
[languages.fi]
dictionary_path = "/usr/share/voikko"   # optional: another voikko-fi (mor.vfst or its directory)
accept = ["Omakanta"]                   # words accepted only in Finnish text

[languages.sv]
# dictionary_path = "dictionaries/sv_FI.aff"   # optional: a Hunspell .aff (its .dic beside it)

[languages.de]
dictionary_path = "dictionaries/de"     # any language with a Hunspell index.aff / index.dic
```

`dictionary_path` is relative to the config root; its contents are part of the results cache
key. `explicit watch` reloads a dictionary or vocab file (`prose.vocab_files`) when its contents
change and re-checks everything. A configured Voikko dictionary also works in builds without the `voikko` feature.

`[[vocab]]` and `[[entity]]` entries apply to every language unless they name one: with
`lang = "fi"` (or `langs = ["fi", "sv"]`) a term is accepted in Finnish text (files, stretches,
marked regions) and still flagged in English. `[languages.fi] accept` adds plain words the
same way.

```toml
[[vocab]]
term = "Omakanta"
description = "Finnish patient portal for health records"
lang = "fi"
```

```toml
[[overrides]]
paths = ["docs/fi/**"]       # gitignore-style globs, relative to the config root
language = "fi"              # or "none"
# dialect = "british"        # replaces prose.dialect
# accept = ["Kalevala"]      # appended to prose.accept
# vocab = [{ term = "Kalevala", description = "Finnish national epic" }]  # appended to [[vocab]]
# rules = { "md/line-length" = "off" }  # merged over [rules]
```

### Gettext

`.po` and `.pot` files are parsed as GNU gettext catalogs (own parser: multi-line strings,
escapes, `#~` obsolete entries, `#|` previous strings, flags, header fields). Findings point at
the exact bytes inside a string, also across concatenated lines.

| Rule | Default | Checks |
|---|---|---|
| `gettext/syntax` | error | Unterminated strings, unknown escapes and keywords, stray text, missing `msgstr`, `msgid_plural` without `msgstr[N]` |
| `gettext/duplicate` | error | The same `msgctxt` + `msgid` twice |
| `gettext/header` | error/warning | Header entry present, `Plural-Forms` parses and stays below `nplurals`, charset (non-UTF-8 is a warning), `Language` set and matching the path (`fi/LC_MESSAGES/x.po`, `fi.po`, `fi/messages.po`) |
| `gettext/plural-count` | error | Translated plural entries have `nplurals` forms |
| `gettext/placeholders` | error | Each `msgstr` keeps the source placeholders: printf (`%s`, `%1$d`, reordering allowed), Python `%(name)s` and `{name}`, ICU `{count, plural, …}` arguments, Elixir/Ruby `%{name}`, Ruby `%<name>s`, Qt `%1`; chosen by the `*-format` flag, otherwise detected from the msgid |
| `gettext/plural-placeholder` | off | A plural `msgstr[N]` drops the count placeholder (`msgstr[0] "Yksi tiedosto"` for `%{count} files`), which many languages do |
| `gettext/markup` | warning | Same HTML/XML tags, Markdown links and code spans |
| `gettext/whitespace` | warning | Same leading/trailing newline and space |
| `gettext/punctuation` | info | Same final `.` `:` `?` `!` `…` (full-width forms count as equal) |
| `gettext/accelerator` | warning | Same number of `&` / `_` keyboard accelerators, when at least 3 msgids use them |
| `gettext/untranslated` | info | Empty `msgstr` (not reported in English catalogs with source-text msgids) |
| `gettext/fuzzy` | warning | `#, fuzzy` entries, which are ignored at runtime |
| `gettext/obsolete` | info | `#~` entries |
| `gettext/same-as-source` | info | A translation identical to a msgid of 3+ words (not in English catalogs, not for capitalized brand names) |

POT templates only get the syntax, duplicate and header checks. Entries whose msgid is a lookup
key (`auth.login.title`) skip the source/translation comparisons.

msgids, `msgid_plural`s, translator comments (`#`) and extracted comments (`#.`) are English
prose: spelling, grammar, `prose/*`, `slop/*` and `style/*` rules check them, with
placeholders, tags and escapes blanked. When a PO file has a POT template next to it
(`priv/gettext/default.pot` for `priv/gettext/fi/LC_MESSAGES/default.po`, or a single `.pot` in
the same directory), msgids and extracted comments are checked in the template only, so each
finding appears once. Translations (`msgstr`, `msgstr[N]`) are prose in the catalog's language,
taken from the header `Language` or else the path (`fi/LC_MESSAGES/x.po`, `fi.po`): English
catalogs get the English rules, Finnish and Swedish ones their spellers (placeholders and markup
blanked as in msgids). Translations in other languages, and copies of the source text, are
skipped.

## Suppressing findings

In Markdown:

```markdown
<!-- explicit-disable-next-line spelling -->
<!-- explicit-disable slop/* -->
...
<!-- explicit-enable -->
<!-- explicit-disable-file md/line-length -- reason goes after two dashes -->
```

In code, put the same directives in any comment: `// explicit-disable-line grammar/*`. In PO
files, use a translator comment: `# explicit-disable-next-line gettext/fuzzy`.

To skip a code block's syntax check, add `skip-lint` to the fence: ```` ```json skip-lint ````.

## Rule families

| Family | What it checks |
|---|---|
| `md/*` | Markdown structure (markdownlint MD001–MD060 equivalents, GitHub alerts, task lists) |
| `spelling`, `grammar/*` | Spelling and grammar, one id per grammar rule |
| `slop/*` | AI slop phrases, mannerisms and narrating comments |
| `prose/*` | Inclusive language, simplification, terminology, readability |
| `links/*` | Local files, anchors, references, footnotes, http status, remote images, same-repo URLs |
| `codeblock/*`, `diagram/*` | JSON, TOML, YAML, DOT, Mermaid and D2 blocks |
| `docs/*` | TOC sync, includes, orphan pages |
| `gettext/*` | PO/POT syntax, headers, plural forms, placeholders, markup, untranslated and fuzzy entries |
| `style/*` | Your own `[[style]]` rules |

Run `explicit rules` for the full list with descriptions.

## Pre-commit and CI

With [devenv](https://devenv.sh), the git hook in `devenv.nix` runs `explicit check --offline`
on staged Markdown and source files. In GitHub Actions, `--format github` produces inline
annotations and `--format sarif` works with code scanning.

## Development

```console
devenv shell
cargo test
cargo test --no-default-features --features mermaid   # without optional features
cargo test --features harper,voikko,swedish           # the full build
scripts/update-linguist.sh       # refresh the embedded GitHub language list
scripts/update-harper-words.sh   # regenerate dictionaries/harper after bumping harper-core
```

The comment rules in `src/rules/slop/comments.rs` are ported from aislop (MIT); see [NOTICE](NOTICE).

## License

Licensed under the [GNU General Public License v3.0 or later](LICENSE) (`GPL-3.0-or-later`), which lets explicit embed and link GPL-licensed language data such as Voikko for Finnish. Releases up to v0.2.0 were published under MIT OR Apache-2.0.
Third-party code and data are listed in [NOTICE](NOTICE).
