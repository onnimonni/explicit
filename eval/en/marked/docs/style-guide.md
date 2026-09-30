# Documentation style guide

This guide applies to everything under ⟪path|docs/⟫ and to doc comments in the code. It is short
on purpose. ⟪informal|Nobody reads long style guides, including the people who write them.⟫

## Voice

- Second person, present tense. "You configure" not "the user will configure".
- Active voice by default. Passive is fine when the actor does not matter: "connections are
  dropped after ⟪unit|30s⟫".
- American spelling: ⟪correct|color⟫, ⟪correct|behavior⟫, ⟪correct|organize⟫. The linter flags the
  British variants, but only in prose, not in code spans like ⟪code|`colour`⟫.
- Contractions are fine. "Don't" reads better than "do not" in a tutorial ⟦then_than|then|than⟧
  in a reference page, but neither is wrong.

## Words

| Use | Not |
|---|---|
| ⟪table|config file⟫ | ⟪table|configuration file, conf⟫ |
| ⟪table|upstream⟫ | ⟪table|backend, origin⟫ |
| ⟪table|route⟫ | ⟪table|rule, mapping⟫ |
| ⟪table|log in (verb), login (noun)⟫ | ⟪table|login (verb)⟫ |

Product names keep ⟪correct|their⟫ official capitalization: ⟪product|GitHub⟫, ⟪product|macOS⟫,
⟪product|PostgreSQL⟫, ⟪product|JavaScript⟫, ⟪product|kubectl⟫, ⟪product|npm⟫, ⟪product|nginx⟫. When
a name starts a sentence and is officially lowercase, rewrite the sentence. Writing
"Kubectl" is worse than reordering.

## Structure

One idea per paragraph, ⟪unit|3⟫ to ⟪unit|5⟫ sentences. Headings are sentence case. Lists are for
parallel items, not for prose that happens to have three parts. Tables are for
data with columns, not for layout. Keep the ⟦british|organisation|organization⟧ flat: two heading levels
below the title, rarely three.

Code blocks always have a language. Use ⟪code|`console`⟫ for shell sessions and ⟪code|`text`⟫ for
output. ⟦fragment|Never a bare fence.|Never use a bare fence.⟧

## Examples

Every option gets an example. Examples should be realistic: ⟪code|`api.example.com`⟫ not
⟪code|`foo`⟫, ⟪unit|30s⟫ not ⟪unit|12345ms⟫. If an example would not work when pasted, mark it
with a comment saying so.

## Links

Link the first mention of another page, not every mention. Relative links, never absolute
⟪url|https://docs.example.com/...⟫ URLs; the linter checks that ⟪correct|their⟫ targets resolve.
External links get checked weekly and a broken one blocks the docs build.

## What we do not do

- ⟪list|Emoji in headings or prose⟫
- ⟪list|"Simply", "just", "obviously"⟫
- ⟪list|Screenshots of terminals; paste the text⟫
- ⟪list|Marketing language in reference docs⟫

The linter enforces most of this. When it flags something you think is right, ⟪correct|it's⟫
usually you; when ⟪correct|your⟫ sure it is the linter, add the word to ⟪path|explicit.toml⟫
with a comment and move on. ⟦punctuation|Do not disable the rule for the whole file, that hides the next real mistake.|Do not disable the rule for the whole file; that hides the next real mistake.⟧
