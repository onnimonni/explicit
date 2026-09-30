# @acme/sdk

Official typescript client for the Acme API. Works in
Node.js 18+, Bun, Deno and modern browsers.

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
dont need to import from deep paths. Types are generated from the
OpenAPI spec, than hand-edited where the spec is too loose.

## Errors

Failed requests throw `AcmeError`. It has a `status`, a machine readable
`code` and the raw response body. Rate limited requests are retried automatically
with exponential backoff; you can turn this off with
`{ retries: 0 }`. The retry loop give up after five attempts or
30s, whichever comes first.

Network errors are wrapped AcmeError with `code: "network"` so
you can handle them the same way as HTTP errors.

## Pagination

List endpoints return an iterator that fetches pages lazily:

```ts
for await (const invoice of acme.invoices.list({ limit: 100 })) {
  console.log(invoice.id);
}
```

It's safe to break out of the loop early; no further requests are made.

## Webhooks

`acme.webhooks.verify(payload, signature, secret)` checks the HMAC signature and
returns the parsed event. Always verify before you trust their contents.
Replay protection is you're responsibility; the SDK does not track event ids.

## Browser usage

Do not ship you're secret key to the browser. Use a short-lived session token
from your backend instead. The bundle is about 9kB gzipped and has
no dependencies.

## Support

Open an issue on github or email sdk@acme.example.
