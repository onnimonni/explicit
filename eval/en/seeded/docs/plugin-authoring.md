# Writing a plugin

Plugins are WebAssembly modules that run on every request of a route. They can read and
rewrite headers, short-circuit with a response, or add fields to the access log. They cannot
read bodies; that is deliberate and unlikley to change.

## Toolchain

Any language that targets `wasm32-wasip2` works. We ship bindings for Rust and
Go (via TinyGo); AssemblyScript is community maintained. The
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

Build with `cargo build --release --target wasm32-wasip2` and reference the .wasm file
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
If your plugin needs shared state, use the `kv` host function, which is
backed by the same store as rate limits.

`on_request` runs before routing decisions are final; `on_response` runs
after the upstream answers. Returning `Verdict::Respond(resp)` from `on_request` skips
the upstream entirely, which is how the maintenance-page example works.

## Limits

| Resource | Limit |
|---|---|
| Fuel per request | 10M instructions |
| Memory | 16MiB |
| Wall clock | 5ms |

A plugin that exceeds an limit is trapped and the request continues as if the plugin
had returned `Continue`, with a error logged. It fails open, by design.
Set `plugins.fail_closed = true` if your plugin is a security control.

## Host functions

- `log(level, msg)`
- `kv_get(key) -> Option<bytes>`, `kv_set(key, bytes, ttl)`
- `now() -> u64` (milliseconds since epoch)
- `random(n) -> bytes`

There is no network access from plugins. People ask every month; the answer stays no.

## Testing

gateway-plugin ships a test harness. `gateway_plugin::test::request("GET", "/")`
builds a request; call your plugin and assert on the result. The harness enforces
the same fuel and memory limits as production, so a plugin that passes it's tests will
not trap for resource reasons in production unless input is very different.

## Publishing

Push the .wasm to any OCI registry with oras and reference it as
`oci://ghcr.io/you/plugin:1.0`. The gateway pulls at config load and verifies the digest
if one is given. We we recommend pinning digests in production.
