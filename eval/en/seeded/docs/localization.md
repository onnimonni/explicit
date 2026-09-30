# Localization

The gateway itself has no user-facing strings beyond error pages, but the admin dashboard and
the documentation are translated. This page explains how, and what the Nordic customers in
particular expect.

## Languages

| Code | Language | Owner |
|---|---|---|
| `en-US` | English (source) | docs team |
| `fi` | Finnish | Sanna Korhonen |
| `sv` | Swedish | Ingrid Nordström |
| `de` | German | agency |

Source strings are American English. Localised files live under locales/<code>/
and are ICU MessageFormat. Do not edit them by hand; they are exported from the translation tool.

## Names and places

Customer names appear in examples and screenshots. Keep them as there owners
write them, including diacritics: "Väinö Mäkelä", "Åsa Sjöberg", "Hämeenlinna",
"Jyväskylä", "Göteborg". The English docs write Gothenburg for the city in
running text but keep "Göteborg" when quoting an address. Company names such as
"Oy Esimerkki Ab" and "Exempel AB" are never translated.

The linter's spell checker is told about these names in explicit.toml; if you add a new
one, add it there too or the docs build fails.

## Dates, numbers, units

Documentation uses ISO 8601 dates (2026-03-17) and SI or IEC units
(10ms, 4GB, 256MiB). Translations may localize the date format but must not
change units. A Finnish reader expects 17.3.2026; a Swedish one expects
2026-03-17; the tooling handles both.

Decimal separators follow the locale in the dashboard. In the docs we write 1.5s
whether the page is English or not, because the value appears in config
examples that must be pasteable.

## Error pages

The built-in `502` and `503` pages honor `Accept-Language` and fall back to
English. It's a plain template; operators can replace it with
`server.error_pages = "./pages/"`. Templates receive `{{status}}`, `{{request_id}}`
and `{{lang}}`.

## Process

1. Strings are frozen a week before release. Translaters get the diff.
2. Translations land in a single PR per language, reviewed by the owner above.
3. Missing translations fall back to English at runtime; the the build does not
   fail, but the release notes list the gaps.

Machine translation is used for first drafts only.
It is fine for "click here" and terrible for anything about certificates.

## Right-to-left

Not supported in the dashboard yet. arabic and Hebrew requests have come up
twice; if a third customer asks, we will schedule it.
