# Writing a plugin

Plugins are ⟪acronym|WebAssembly⟫ modules that run on every request of a route. They can read and
rewrite headers, short-circuit with a response, or add fields to the access log. They cannot
read bodies; that is deliberate and ⟦spelling|unlikley|unlikely⟧ to change.

## Toolchain

Any language that targets ⟪code|`wasm32-wasip2`⟫ works. We ship bindings for ⟪product|Rust⟫ and
Go (via ⟪product|TinyGo⟫); ⟪product|AssemblyScript⟫ is community maintained. The
examples below are Rust.

```console
cargo new --lib add-header
cargo add gateway-plugin
```

## A minimal plugin

```rust
use gateway_plugin::{Plugin, Request, Verdict};

struct AddHeader;

impl Plugin for AddHeader {
    fn on_request(&self, req: &mut Request) -> Verdict {
        req.headers_mut().insert("x-served-by", "gateway");
        Verdict::Continue
    }
}

gateway_plugin::export!(AddHeader);
```

Build with ⟪code|`cargo build --release --target wasm32-wasip2`⟫ and reference the ⟪path|.wasm⟫ file
from the route:

```toml
[[route]]
path = "/"
upstream = "http://backend:8080"
plugins = ["./target/wasm32-wasip2/release/add_header.wasm"]
```

## Lifecycle

The module is instantiated once per worker thread at config load and reused for every request.
Keep state out of globals; each worker has its own instance and they do not see each other.
If ⟪correct|your⟫ plugin needs shared state, use the ⟪code|`kv`⟫ host function, which is
backed by the same store as rate limits.

⟪code|`on_request`⟫ runs before routing decisions are final; ⟪code|`on_response`⟫ runs
after the upstream answers. Returning ⟪code|`Verdict::Respond(resp)`⟫ from ⟪code|`on_request`⟫ skips
the upstream entirely, which is how the ⟪product|maintenance-page⟫ example works.

## Limits

| Resource | Limit |
|---|---|
| ⟪table|Fuel per request⟫ | ⟪unit|10M instructions⟫ |
| ⟪table|Memory⟫ | ⟪unit|16MiB⟫ |
| ⟪table|Wall clock⟫ | ⟪unit|5ms⟫ |

A plugin that exceeds ⟦a_an|an limit|a limit⟧ is trapped and the request continues as if the plugin
had returned ⟪code|`Continue`⟫, with ⟦a_an|a error|an error⟧ logged. It fails open, by design.
Set ⟪code|`plugins.fail_closed = true`⟫ if ⟪correct|your⟫ plugin is a security control.

## Host functions

- ⟪code|`log(level, msg)`⟫
- ⟪code|`kv_get(key) -> Option<bytes>`⟫, ⟪code|`kv_set(key, bytes, ttl)`⟫
- ⟪code|`now() -> u64`⟫ (milliseconds since epoch)
- ⟪code|`random(n) -> bytes`⟫

There is no network access from plugins. ⟪informal|People ask every month; the answer stays no.⟫

## Testing

⟪product|gateway-plugin⟫ ships a test harness. ⟪code|`gateway_plugin::test::request("GET", "/")`⟫
builds a request; call ⟪correct|your⟫ plugin and assert on the result. The harness enforces
the same fuel and memory limits as production, so a plugin that passes ⟦its_its|it's|its⟧ tests will
not trap for resource reasons in production ⟦missing_extra_word|unless input is|unless the input is⟧ very different.

## Publishing

Push the ⟪path|.wasm⟫ to any ⟪acronym|OCI⟫ registry with ⟪product|oras⟫ and reference it as
⟪code|`oci://ghcr.io/you/plugin:1.0`⟫. The gateway pulls at config load and verifies the digest
if one is given. ⟦repeated_word|We we|We⟧ recommend pinning digests in production.
