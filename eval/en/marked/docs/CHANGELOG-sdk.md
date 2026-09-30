# @acme/sdk changelog

## 3.2.0 (2026-04-02)

- Added ⟪code|`acme.events.list()`⟫ with cursor pagination.
- Added ⟪product|Deno⟫ to the test matrix. ⟪product|Bun⟫ ⟪version|1.2⟫ is now the minimum supported Bun.
- The ⟪code|`retries`⟫ option accepts ⟦a_an|a object|an object⟧ with ⟪code|`max`⟫ and ⟪code|`baseDelay`⟫.
- Fixed a race where two concurrent token refreshes both hit the network. Only one
  ⟦repeated_word|of of|of⟧ them does now; the other waits.

## 3.1.4 (2026-03-14)

- Fixed ⟪code|`AcmeError.status`⟫ being ⟪code|`undefined`⟫ for network errors. It is now ⟪code|`0`⟫,
  which matches what ⟪product|fetch⟫ does in browsers.
- Type of ⟪code|`Invoice.due_at`⟫ corrected from ⟪code|`string`⟫ to ⟪code|`string | null`⟫.
  This is technically a breaking change for TypeScript users, but the
  runtime already returned ⟪code|`null`⟫, so ⟦your_youre|you're|your⟧ code was wrong before and is
  now told so.

## 3.1.3 (2026-02-27)

- The ⟪acronym|ESM⟫ build no longer imports ⟪code|`node:crypto`⟫ at module load time. Browser
  bundles shrink by ⟪unit|3kB⟫ and ⟪product|Cloudflare Workers⟫ stop complaining.
- Webhook verification tolerates a ⟪unit|300s⟫ clock skew instead of ⟪unit|60s⟫. ⟦fragment|Because customer servers have worse clocks than we assumed.|Customer servers have worse clocks than we assumed.⟧

## 3.1.2 (2026-02-10)

- Node.js ⟪version|16⟫ dropped. It has been end of life for over a year.
- Fixed ⟦a_an|an typo|a typo⟧ in the ⟪code|`Customer`⟫ docstring.

## 3.1.1 (2026-01-22)

- Retries now honor ⟪code|`Retry-After`⟫ on ⟪code|`503`⟫, not only on ⟪code|`429`⟫.
- Removed the accidental dependency on ⟪product|lodash⟫. ⟪informal|Nobody knows how it got there.⟫

## 3.1.0 (2026-01-08)

- New ⟪code|`acme.customers.search()`⟫ endpoint wrapper.
- The `timeout` option is now per attempt; previously it was for the whole request including retries.
  Set ⟪code|`retries: 0`⟫ to get the old ⟦british|behaviour|behavior⟧.
