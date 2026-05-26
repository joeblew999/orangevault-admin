//! AdminService implementation. Reads orangevault's D1 tables directly.
//!
//! TODO: gate every RPC on a macaroon verified against MACAROON_ROOT_KEY.
//! For now the worker is unauthenticated — fine for the scaffold, **must**
//! be added before exposing externally.

use connectrpc::{ConnectError, RequestContext, Response, ServiceResult};
use worker::D1Database;
use worker::send::IntoSendFuture;

use crate::proto::orangevault_admin::v1::{
    AdminService, DeleteUserResponse, GetUserResponse, HealthzResponse, ListOrganizationsResponse,
    ListUserMembershipsResponse, ListUsersResponse, Membership, Organization,
    OwnedDeleteUserRequestView, OwnedGetUserRequestView, OwnedHealthzRequestView,
    OwnedListOrganizationsRequestView, OwnedListUserMembershipsRequestView,
    OwnedListUsersRequestView, OwnedRotateSecurityStampRequestView, RotateSecurityStampResponse,
    User,
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
        // D1 rejects i64 — bind LIMIT as f64.
        let limit = request.limit.unwrap_or(100).clamp(1, 1000) as f64;

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

    async fn get_user(
        &self,
        _ctx: RequestContext,
        request: OwnedGetUserRequestView,
    ) -> ServiceResult<GetUserResponse> {
        if request.user_id.is_empty() {
            return Err(ConnectError::invalid_argument("user_id required"));
        }
        let stmt = self
            .db
            .prepare("SELECT uuid, email, name, email_verified, created_at, updated_at FROM users WHERE uuid = ?1")
            .bind(&[request.user_id.into()])
            .map_err(d1_err)?;
        let row: Option<UserRow> = stmt.first(None).into_send().await.map_err(d1_err)?;
        let row = row.ok_or_else(|| ConnectError::not_found("user not found"))?;
        Response::ok(GetUserResponse {
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

    async fn list_organizations(
        &self,
        _ctx: RequestContext,
        request: OwnedListOrganizationsRequestView,
    ) -> ServiceResult<ListOrganizationsResponse> {
        let limit = request.limit.unwrap_or(100).clamp(1, 1000) as f64;
        let stmt = self
            .db
            .prepare(
                "SELECT o.uuid, o.name, o.billing_email, \
                        COUNT(m.uuid) AS member_count \
                 FROM organizations o \
                 LEFT JOIN memberships m ON m.org_uuid = o.uuid \
                 GROUP BY o.uuid \
                 ORDER BY o.name \
                 LIMIT ?1",
            )
            .bind(&[limit.into()])
            .map_err(d1_err)?;
        let rows: Vec<OrgRow> = stmt
            .all()
            .into_send()
            .await
            .map_err(d1_err)?
            .results()
            .map_err(d1_err)?;
        let organizations = rows
            .into_iter()
            .map(|r| Organization {
                id: r.uuid,
                name: r.name,
                billing_email: r.billing_email,
                member_count: r.member_count as u32,
                ..Default::default()
            })
            .collect();
        Response::ok(ListOrganizationsResponse {
            organizations,
            ..Default::default()
        })
    }

    async fn delete_user(
        &self,
        _ctx: RequestContext,
        request: OwnedDeleteUserRequestView,
    ) -> ServiceResult<DeleteUserResponse> {
        let user_id: String = request.user_id.into();
        if user_id.is_empty() {
            return Err(ConnectError::invalid_argument("user_id required"));
        }

        // Verify the user exists first so we 404 cleanly instead of
        // running 11 no-op deletes.
        let exists: Option<UuidRow> = self
            .db
            .prepare("SELECT uuid FROM users WHERE uuid = ?1")
            .bind(&[user_id.clone().into()])
            .map_err(d1_err)?
            .first(None)
            .into_send()
            .await
            .map_err(d1_err)?;
        if exists.is_none() {
            return Err(ConnectError::not_found("user not found"));
        }

        // orangevault's upstream schema doesn't declare ON DELETE CASCADE,
        // so cascade by hand. Order matters: leaves first, root last.
        // Wrapped in a D1 batch so they apply atomically.
        let stmts = [
            "DELETE FROM users_collections WHERE user_uuid = ?1",
            "DELETE FROM favorites WHERE user_uuid = ?1",
            "DELETE FROM folders_ciphers WHERE cipher_uuid IN (SELECT uuid FROM ciphers WHERE user_uuid = ?1)",
            "DELETE FROM attachments WHERE cipher_uuid IN (SELECT uuid FROM ciphers WHERE user_uuid = ?1)",
            "DELETE FROM ciphers WHERE user_uuid = ?1",
            "DELETE FROM folders WHERE user_uuid = ?1",
            "DELETE FROM sends WHERE user_uuid = ?1",
            "DELETE FROM two_factor WHERE user_uuid = ?1",
            "DELETE FROM devices WHERE user_uuid = ?1",
            "DELETE FROM memberships WHERE user_uuid = ?1",
            "DELETE FROM users WHERE uuid = ?1",
        ];

        let mut prepared = Vec::with_capacity(stmts.len());
        for sql in stmts {
            let p = self
                .db
                .prepare(sql)
                .bind(&[user_id.clone().into()])
                .map_err(d1_err)?;
            prepared.push(p);
        }

        let results = self.db.batch(prepared).into_send().await.map_err(d1_err)?;
        let total: u32 = results
            .iter()
            .map(|r| {
                // worker-rs D1Result exposes meta() with .changes; conservatively
                // pull what we can without depending on internal shape.
                r.meta()
                    .ok()
                    .flatten()
                    .and_then(|m| m.changes)
                    .unwrap_or(0)
                    .max(0) as u32
            })
            .sum();

        Response::ok(DeleteUserResponse {
            deleted_rows: total,
            ..Default::default()
        })
    }

    async fn list_user_memberships(
        &self,
        _ctx: RequestContext,
        request: OwnedListUserMembershipsRequestView,
    ) -> ServiceResult<ListUserMembershipsResponse> {
        if request.user_id.is_empty() {
            return Err(ConnectError::invalid_argument("user_id required"));
        }
        let stmt = self
            .db
            .prepare(
                "SELECT m.org_uuid, o.name AS org_name, m.atype, m.status \
                 FROM memberships m \
                 JOIN organizations o ON o.uuid = m.org_uuid \
                 WHERE m.user_uuid = ?1 \
                 ORDER BY o.name",
            )
            .bind(&[request.user_id.into()])
            .map_err(d1_err)?;
        let rows: Vec<MembershipRow> = stmt
            .all()
            .into_send()
            .await
            .map_err(d1_err)?
            .results()
            .map_err(d1_err)?;
        let memberships = rows
            .into_iter()
            .map(|r| Membership {
                organization_id: r.org_uuid,
                organization_name: r.org_name,
                role: r.atype,
                status: r.status,
                ..Default::default()
            })
            .collect();
        Response::ok(ListUserMembershipsResponse {
            memberships,
            ..Default::default()
        })
    }
}

#[derive(serde::Deserialize)]
struct UuidRow {
    #[allow(dead_code)]
    uuid: String,
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

#[derive(serde::Deserialize)]
struct OrgRow {
    uuid: String,
    name: String,
    billing_email: String,
    member_count: i64,
}

#[derive(serde::Deserialize)]
struct MembershipRow {
    org_uuid: String,
    org_name: String,
    atype: i32,
    status: i32,
}

fn d1_err(e: impl std::fmt::Display) -> ConnectError {
    ConnectError::internal(format!("d1: {e}"))
}
