//! AdminService implementation. Reads orangevault's D1 tables directly.
//!
//! TODO: gate every RPC on a macaroon verified against MACAROON_ROOT_KEY.
//! For now the worker is unauthenticated — fine for the scaffold, **must**
//! be added before exposing externally.

use connectrpc::{ConnectError, RequestContext, Response, ServiceResult};
use worker::D1Database;
use worker::send::IntoSendFuture;

use crate::proto::orangevault_admin::v1::{
    AdminService, HealthzResponse, ListUsersResponse, OwnedHealthzRequestView,
    OwnedListUsersRequestView, OwnedRotateSecurityStampRequestView,
    RotateSecurityStampResponse, User,
};

pub struct AdminServer {
    db: D1Database,
}

impl AdminServer {
    pub fn new(db: D1Database) -> Self {
        Self { db }
    }
}

impl AdminService for AdminServer {
    async fn healthz(
        &self,
        _ctx: RequestContext,
        _request: OwnedHealthzRequestView,
    ) -> ServiceResult<HealthzResponse> {
        Response::ok(HealthzResponse {
            status: "ok".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            ..Default::default()
        })
    }

    async fn list_users(
        &self,
        _ctx: RequestContext,
        request: OwnedListUsersRequestView,
    ) -> ServiceResult<ListUsersResponse> {
        let limit = request.limit.unwrap_or(100).clamp(1, 1000) as i64;

        let stmt = self
            .db
            .prepare("SELECT uuid, email, name, email_verified, created_at, updated_at FROM users ORDER BY created_at DESC LIMIT ?1")
            .bind(&[limit.into()])
            .map_err(d1_err)?;

        let rows: Vec<UserRow> = stmt
            .all()
            .into_send()
            .await
            .map_err(d1_err)?
            .results()
            .map_err(d1_err)?;

        let users = rows
            .into_iter()
            .map(|r| User {
                id: r.uuid,
                email: r.email,
                name: r.name,
                email_verified: r.email_verified != 0,
                created_at: r.created_at,
                updated_at: r.updated_at,
                ..Default::default()
            })
            .collect();

        Response::ok(ListUsersResponse {
            users,
            next_cursor: None,
            ..Default::default()
        })
    }

    async fn rotate_security_stamp(
        &self,
        _ctx: RequestContext,
        request: OwnedRotateSecurityStampRequestView,
    ) -> ServiceResult<RotateSecurityStampResponse> {
        if request.user_id.is_empty() {
            return Err(ConnectError::invalid_argument("user_id required"));
        }

        let new_stamp = uuid::Uuid::new_v4().to_string();

        let stmt = self
            .db
            .prepare("UPDATE users SET security_stamp = ?1, updated_at = datetime('now') WHERE uuid = ?2 RETURNING uuid, email, name, email_verified, created_at, updated_at")
            .bind(&[new_stamp.into(), request.user_id.into()])
            .map_err(d1_err)?;

        let row: Option<UserRow> = stmt.first(None).into_send().await.map_err(d1_err)?;
        let row = row.ok_or_else(|| ConnectError::not_found("user not found"))?;

        Response::ok(RotateSecurityStampResponse {
            user: buffa::MessageField::some(User {
                id: row.uuid,
                email: row.email,
                name: row.name,
                email_verified: row.email_verified != 0,
                created_at: row.created_at,
                updated_at: row.updated_at,
                ..Default::default()
            }),
            ..Default::default()
        })
    }
}

#[derive(serde::Deserialize)]
struct UserRow {
    uuid: String,
    email: String,
    name: String,
    email_verified: i64,
    created_at: String,
    updated_at: String,
}

fn d1_err(e: impl std::fmt::Display) -> ConnectError {
    ConnectError::internal(format!("d1: {e}"))
}
