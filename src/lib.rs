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
    if req.method() == Method::GET && req.uri().path() == "/healthz" {
        return Ok(healthz());
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

fn healthz() -> Response<ConnectRpcBody> {
    Response::builder()
        .status(StatusCode::OK)
        .header(http::header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .body(ConnectRpcBody::Full(Full::new(Bytes::from("ok"))))
        .expect("static response builder inputs are valid")
}
