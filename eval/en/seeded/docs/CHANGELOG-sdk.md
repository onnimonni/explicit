# @acme/sdk changelog

## 3.2.0 (2026-04-02)

- Added `acme.events.list()` with cursor pagination.
- Added Deno to the test matrix. Bun 1.2 is now the minimum supported Bun.
- The `retries` option accepts a object with `max` and `baseDelay`.
- Fixed a race where two concurrent token refreshes both hit the network. Only one
  of of them does now; the other waits.

## 3.1.4 (2026-03-14)

- Fixed `AcmeError.status` being `undefined` for network errors. It is now `0`,
  which matches what fetch does in browsers.
- Type of `Invoice.due_at` corrected from `string` to `string | null`.
  This is technically a breaking change for TypeScript users, but the
  runtime already returned `null`, so you're code was wrong before and is
  now told so.

## 3.1.3 (2026-02-27)

- The ESM build no longer imports `node:crypto` at module load time. Browser
  bundles shrink by 3kB and Cloudflare Workers stop complaining.
- Webhook verification tolerates a 300s clock skew instead of 60s. Because customer servers have worse clocks than we assumed.

## 3.1.2 (2026-02-10)

- Node.js 16 dropped. It has been end of life for over a year.
- Fixed an typo in the `Customer` docstring.

## 3.1.1 (2026-01-22)

- Retries now honor `Retry-After` on `503`, not only on `429`.
- Removed the accidental dependency on lodash. Nobody knows how it got there.

## 3.1.0 (2026-01-08)

- New `acme.customers.search()` endpoint wrapper.
- The `timeout` option is now per attempt; previously it was for the whole request including retries.
  Set `retries: 0` to get the old behaviour.
