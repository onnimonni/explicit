# Documentation style guide

This guide applies to everything under docs/ and to doc comments in the code. It is short
on purpose. Nobody reads long style guides, including the people who write them.

## Voice

- Second person, present tense. "You configure" not "the user will configure".
- Active voice by default. Passive is fine when the actor does not matter: "connections are
  dropped after 30s".
- American spelling: color, behavior, organize. The linter flags the
  British variants, but only in prose, not in code spans like `colour`.
- Contractions are fine. "Don't" reads better than "do not" in a tutorial then
  in a reference page, but neither is wrong.

## Words

| Use | Not |
|---|---|
| config file | configuration file, conf |
| upstream | backend, origin |
| route | rule, mapping |
| log in (verb), login (noun) | login (verb) |

Product names keep their official capitalization: GitHub, macOS,
PostgreSQL, JavaScript, kubectl, npm, nginx. When
a name starts a sentence and is officially lowercase, rewrite the sentence. Writing
"Kubectl" is worse than reordering.

## Structure

One idea per paragraph, 3 to 5 sentences. Headings are sentence case. Lists are for
parallel items, not for prose that happens to have three parts. Tables are for
data with columns, not for layout. Keep the organisation flat: two heading levels
below the title, rarely three.

Code blocks always have a language. Use `console` for shell sessions and `text` for
output. Never a bare fence.

## Examples

Every option gets an example. Examples should be realistic: `api.example.com` not
`foo`, 30s not 12345ms. If an example would not work when pasted, mark it
with a comment saying so.

## Links

Link the first mention of another page, not every mention. Relative links, never absolute
https://docs.example.com/... URLs; the linter checks that their targets resolve.
External links get checked weekly and a broken one blocks the docs build.

## What we do not do

- Emoji in headings or prose
- "Simply", "just", "obviously"
- Screenshots of terminals; paste the text
- Marketing language in reference docs

The linter enforces most of this. When it flags something you think is right, it's
usually you; when your sure it is the linter, add the word to explicit.toml
with a comment and move on. Do not disable the rule for the whole file, that hides the next real mistake.
