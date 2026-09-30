# Localization

The gateway itself has no user-facing strings beyond error pages, but the admin dashboard and
the documentation are translated. This page explains how, and what the Nordic customers in
particular expect.

## Languages

| Code | Language | Owner |
|---|---|---|
| ⟪code|`en-US`⟫ | ⟪table|English (source)⟫ | ⟪table|docs team⟫ |
| ⟪code|`fi`⟫ | ⟪table|Finnish⟫ | ⟪name|Sanna Korhonen⟫ |
| ⟪code|`sv`⟫ | ⟪table|Swedish⟫ | ⟪name|Ingrid Nordström⟫ |
| ⟪code|`de`⟫ | ⟪table|German⟫ | ⟪table|agency⟫ |

Source strings are American English. ⟦british|Localised|Localized⟧ files live under ⟪path|locales/<code>/⟫
and are ⟪acronym|ICU⟫ MessageFormat. Do not edit them by hand; they are exported from the translation tool.

## Names and places

Customer names appear in examples and screenshots. Keep them as ⟦their_there|there|their⟧ owners
write them, including diacritics: "⟪name|Väinö Mäkelä⟫", "⟪name|Åsa Sjöberg⟫", "⟪name|Hämeenlinna⟫",
"⟪name|Jyväskylä⟫", "⟪name|Göteborg⟫". The English docs write ⟪name|Gothenburg⟫ for the city in
running text but keep "⟪name|Göteborg⟫" when quoting an address. Company names such as
"⟪name|Oy Esimerkki Ab⟫" and "⟪name|Exempel AB⟫" are never translated.

The linter's spell checker is told about these names in ⟪path|explicit.toml⟫; if you add a new
one, add it ⟪correct|there⟫ too or the docs build fails.

## Dates, numbers, units

Documentation uses ⟪acronym|ISO⟫ 8601 dates (⟪unit|2026-03-17⟫) and ⟪acronym|SI⟫ or ⟪acronym|IEC⟫ units
(⟪unit|10ms⟫, ⟪unit|4GB⟫, ⟪unit|256MiB⟫). Translations may localize the date format but must not
change units. ⟪correct|A Finnish⟫ reader expects ⟪unit|17.3.2026⟫; a Swedish one expects
⟪unit|2026-03-17⟫; the tooling handles both.

Decimal separators follow the locale in the dashboard. In the docs we write ⟪unit|1.5s⟫
⟪correct|whether⟫ the page is English or not, because the value appears in config
examples that must be pasteable.

## Error pages

The built-in ⟪code|`502`⟫ and ⟪code|`503`⟫ pages honor ⟪code|`Accept-Language`⟫ and fall back to
English. ⟪correct|It's⟫ a plain template; operators can replace it with
⟪code|`server.error_pages = "./pages/"`⟫. Templates receive ⟪code|`{{status}}`⟫, ⟪code|`{{request_id}}`⟫
and ⟪code|`{{lang}}`⟫.

## Process

1. Strings are frozen a week before release. ⟦spelling|Translaters|Translators⟧ get the diff.
2. Translations land in a single PR per language, reviewed by the owner above.
3. Missing translations fall back to English at runtime; ⟦repeated_word|the the|the⟧ build does not
   fail, but the release notes list the gaps.

Machine translation is used for first drafts only.
⟪informal|It is fine for "click here" and terrible for anything about certificates.⟫

## Right-to-left

Not supported in the dashboard yet. ⟦capitalization|arabic|Arabic⟧ and Hebrew requests have come up
twice; if a third customer asks, we will schedule it.
