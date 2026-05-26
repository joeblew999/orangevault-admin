//! Admin worker for orangevault.
//!
//! Reads orangevault's `users` table (via the same D1 binding) and exposes
//! a small ConnectRPC AdminService. Auth is a bearer admin macaroon stored
//! in the keychain on the operator's machine.
//!
//! Routes: every `/orangevault_admin.v1.AdminService/*` path is dispatched
//! to the ConnectRPC stack. `/healthz` short-circuits for liveness.

#![allow(refining_impl_trait)]

use std::sync::Arc;

use bytes::Bytes;
use connectrpc::{ConnectRpcBody, ConnectRpcService, Router as RpcRouter};
use http::{Method, Response, StatusCode};
use http_body_util::Full;
use tower::Service;
use worker::{Context, Env, HttpRequest, event};

pub(crate) mod proto {
    connectrpc::include_generated!();
}

pub mod services;

use crate::proto::orangevault_admin::v1::AdminServiceExt;
use crate::services::AdminServer;

#[event(fetch, respond_with_errors)]
async fn fetch(
    req: HttpRequest,
    env: Env,
    _ctx: Context,
) -> worker::Result<http::Response<ConnectRpcBody>> {
    // Healthz is intentionally unauthenticated for liveness probes.
    if req.method() == Method::GET && req.uri().path() == "/healthz" {
        return Ok(healthz());
    }

    // Bearer-token gate. ADMIN_TOKEN is set as a wrangler secret on the
    // deployed worker; reject every other request without it. Upgrade
    // path is libmacaroon — already a Cargo dep, just not wired yet.
    let expected = env.secret("ADMIN_TOKEN").ok().map(|s| s.to_string());
    match (expected, bearer_token(&req)) {
        // No ADMIN_TOKEN configured on the worker → fail closed.
        (None, _) => return Ok(unauthorized("ADMIN_TOKEN not configured on worker")),
        (Some(want), Some(got)) if constant_time_eq(want.as_bytes(), got.as_bytes()) => {
            // pass
        }
        _ => return Ok(unauthorized("missing or invalid bearer token")),
    }

    let db = env.d1("DB")?;
    let server = Arc::new(AdminServer::new(db));

    let router = RpcRouter::new();
    let router = server.register(router);

    let mut svc = ConnectRpcService::new(router);
    svc.call(req)
        .await
        .map_err(|e| worker::Error::RustError(format!("rpc dispatch: {e}")))
}

fn bearer_token(req: &HttpRequest) -> Option<String> {
    let auth = req.headers().get(http::header::AUTHORIZATION)?;
    let s = auth.to_str().ok()?;
    s.strip_prefix("Bearer ").map(|t| t.trim().to_string())
}

/// Constant-time byte comparison. Worth ~50 LoC of crate-dep avoidance for
/// a 32-char token: same length, then XOR-or accumulator.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff: u8 = 0;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

fn unauthorized(msg: &str) -> Response<ConnectRpcBody> {
    let body = format!("{{\"code\":\"unauthenticated\",\"message\":\"{msg}\"}}");
    Response::builder()
        .status(StatusCode::UNAUTHORIZED)
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(ConnectRpcBody::Full(Full::new(Bytes::from(body))))
        .expect("static response builder inputs are valid")
}

fn healthz() -> Response<ConnectRpcBody> {
    Response::builder()
        .status(StatusCode::OK)
        .header(http::header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .body(ConnectRpcBody::Full(Full::new(Bytes::from("ok"))))
        .expect("static response builder inputs are valid")
}
