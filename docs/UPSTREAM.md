# Upstream dependency tracking

A pin is a **snapshot**, not a freeze. Upgrade when an upstream moves and it
buys us something. This file is the map.

_Last checked: 2026-10-22._ Cross-repo rollout tracker: [cf-connectrpc-middleware#4](https://github.com/joeblew999/cf-connectrpc-middleware/issues/4).

## Crates

| Dep | We pin | Latest published | Source | Notes |
|---|---|---|---|---|
| `connectrpc` | **0.9.1** (`json`) | 0.9.1 (2026-09-21) | [anthropics/connect-rust](https://github.com/anthropics/connect-rust) | `default-features = false` + `features = ["json"]`. connect-web speaks JSON, and without the feature (opt-in since 0.8) every call is **HTTP 415**. |
| `connectrpc-build` | **0.9.0** | 0.9.0 (2026-08-25) | anthropics/connect-rust | build.rs codegen; moves with `connectrpc`. |
| `buffa` / `buffa-types` | **0.9.2** | 0.9.2 (2026-09-03) | [anthropics/buffa](https://github.com/anthropics/buffa) | Lockstep with `connectrpc`. |
| `worker` / `worker-macros` | **0.8.7** (`http`, `d1`) | 0.8.7 (2026-09-25) | [cloudflare/workers-rs](https://github.com/cloudflare/workers-rs) | Needs `rust-version` ≥ 1.91. `wrangler.toml` installs `worker-build@^0.8` (= 0.8.7), which needs **wasm-bindgen ≥ 0.2.125** in the lock (on 0.2.129). `mise run deps:update` refreshes it. |
| `libmacaroon` | **0.2** (lock 0.2.1) | **0.3.0** ⚠️ | [crates.io/libmacaroon](https://crates.io/crates/libmacaroon) | Auth tokens. **One minor behind.** Evaluate 0.3 together with example-multitenant-worker, which uses the same pattern. |
| `tower` | `0.5` | 0.5.3 | [tower-rs/tower](https://github.com/tower-rs/tower) | |

## Web UI (`web/`)

| Dep | We pin | Source |
|---|---|---|
| `@connectrpc/connect` / `-web` | `^2.0.0` | [connectrpc/connect-es](https://github.com/connectrpc/connect-es) |
| `@bufbuild/protobuf` / `protoc-gen-es` | `^2.2.0` | [bufbuild/protobuf-es](https://github.com/bufbuild/protobuf-es) |
| `@bufbuild/buf` | `^1.50.0` | [bufbuild/buf](https://github.com/bufbuild/buf) |
| `vite` | `^6.0.0` | [vitejs/vite](https://github.com/vitejs/vite) |

## Relationships

- **[orangevault](https://github.com/joeblew999/orangevault)** (our fork of connyay/orangevault): we read **its D1 database directly**. Its schema is our real upstream contract, so a schema migration there can break our SQL. Check its migrations when it moves.
- Layout follows connyay's [example-multitenant-worker](https://github.com/connyay/example-multitenant-worker).
- No crate links against us, and we don't link [cf-connectrpc-middleware](https://github.com/joeblew999/cf-connectrpc-middleware). We track its connectrpc line only to share one set of gotchas, so a bump here can land any time.

## Upgrade checklist

1. Bump the pins, then `cargo update` (wasm-bindgen ≥ 0.2.125).
2. `cargo check` + clippy `-D warnings` on wasm32.
3. `worker-build --release` (the real deploy path).
4. `wrangler dev --local` + a JSON `Healthz` request (catches a dropped `json` feature).
