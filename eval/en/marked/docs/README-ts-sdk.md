# @acme/sdk

Official ⟦capitalization|typescript|TypeScript⟧ client for the Acme ⟪acronym|API⟫. Works in
⟪product|Node.js⟫ ⟪version|18+⟫, ⟪product|Bun⟫, ⟪product|Deno⟫ and modern browsers.

```console
bun add @acme/sdk
```

## Getting started

```ts
import { Acme } from "@acme/sdk";

const acme = new Acme({ apiKey: process.env.ACME_KEY });
const user = await acme.users.get("usr_123");
```

The client is fully typed. Every response type is exported from the package root, so you
⟦spelling|dont|don't⟧ need to import from deep paths. Types are generated from the
⟪product|OpenAPI⟫ spec, ⟦then_than|than|then⟧ hand-edited where the spec is too loose.

## Errors

Failed requests throw ⟪code|`AcmeError`⟫. It has a ⟪code|`status`⟫, a machine readable
⟪code|`code`⟫ and the raw response body. Rate limited requests are retried automatically
with exponential backoff; you can turn this off with
⟪code|`{ retries: 0 }`⟫. The retry loop ⟦agreement|give|gives⟧ up after five attempts or
⟪unit|30s⟫, whichever comes first.

Network errors are ⟦missing_extra_word|wrapped AcmeError|wrapped in an AcmeError⟧ with ⟪code|`code: "network"`⟫ so
you can handle them the same way as ⟪acronym|HTTP⟫ errors.

## Pagination

List endpoints return ⟪correct|an iterator⟫ that fetches pages lazily:

```ts
for await (const invoice of acme.invoices.list({ limit: 100 })) {
  console.log(invoice.id);
}
```

⟪correct|It's⟫ safe to break out of the loop early; no further requests are made.

## Webhooks

⟪code|`acme.webhooks.verify(payload, signature, secret)`⟫ checks the ⟪acronym|HMAC⟫ signature and
returns the parsed event. Always verify before you trust ⟪correct|their⟫ contents.
Replay protection is ⟦your_youre|you're|your⟧ responsibility; the SDK does not track event ids.

## Browser usage

Do not ship ⟦your_youre|you're|your⟧ secret key to the browser. Use a short-lived session token
from ⟪correct|your⟫ backend instead. The bundle is about ⟪unit|9kB⟫ gzipped and has
no dependencies.

## Support

Open an issue on ⟦capitalization|github|GitHub⟧ or email ⟪url|sdk@acme.example⟫.
