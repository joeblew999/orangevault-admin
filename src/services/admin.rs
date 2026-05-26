//! AdminService implementation. Reads orangevault's D1 tables directly.
//!
//! TODO: gate every RPC on a macaroon verified against MACAROON_ROOT_KEY.
//! For now the worker is unauthenticated — fine for the scaffold, **must**
//! be added before exposing externally.

use connectrpc::{ConnectError, RequestContext, Response, ServiceResult};
use worker::D1Database;
use worker::send::IntoSendFuture;

use crate::proto::orangevault_admin::v1::{
    AdminService, GetUserResponse, HealthzResponse, ListOrganizationsResponse,
    ListUserMembershipsResponse, ListUsersResponse, Membership, Organization,
    OwnedGetUserRequestView, OwnedHealthzRequestView, OwnedListOrganizationsRequestView,
    OwnedListUserMembershipsRequestView, OwnedListUsersRequestView,
    OwnedRotateSecurityStampRequestView, RotateSecurityStampResponse, User,
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
