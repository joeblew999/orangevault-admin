# orangevault-admin

Admin worker for [orangevault](https://github.com/joeblew999/orangevault). Reads the same D1 instance directly; exposes a small ConnectRPC `AdminService` for management ops (list users, rotate keys, …) that orangevault itself doesn't surface.

Built on connyay's stack — see [`example-multitenant-worker`](https://github.com/connyay/example-multitenant-worker) for the reference layout:

| Layer | Crate |
|---|---|
| Wire protocol | [`connectrpc`](https://crates.io/crates/connectrpc) |
| Protobuf runtime + codegen | [`buffa`](https://crates.io/crates/buffa) / `connectrpc-build` |
| Auth tokens | [`libmacaroon`](https://crates.io/crates/libmacaroon) |
| Runtime | Cloudflare Workers (`worker-rs` → WASM) |

## Initial RPCs

- `Healthz` — liveness
- `ListUsers` — paginated list of orangevault users (uuid, email, name, …)
- `RotateSecurityStamp` — invalidate a user's outstanding access tokens (orangevault re-checks `sstamp` on every authenticated request)

More to come: `DisableUser`, `ListOrganizations`, `ListSends`, `PurgeTrashed`, etc.

## Architecture

```
                                D1 (shared with orangevault)
                                       ▲
                                       │
  ┌─────────────┐   ConnectRPC   ┌─────┴──────────┐
  │  admin CLI  │ ───────────────▶│ orangevault-   │
  │  (gen'd)    │   bearer        │ admin (Worker) │
  └─────────────┘   macaroon      └────────────────┘
```

Same D1 instance is also bound by [orangevault](https://github.com/joeblew999/orangevault) — admin reads via SQL while the main worker handles user-facing Bitwarden API traffic.

## Status

Scaffold only. `RotateSecurityStamp` writes to D1 but **no auth middleware yet** — must add macaroon verification before opening this worker to the network. Tracked in `src/services/admin.rs` TODO.

## Spin-up

```bash
# Toolchain
mise install
rustup target add wasm32-unknown-unknown

# Set required secrets in fnox keychain (one-time)
fnox set --global -p keychain CLOUDFLARE_API_TOKEN
fnox set --global -p keychain CLOUDFLARE_ACCOUNT_ID

# Local
mise run dev

# Deploy
mise run deploy
```
