//! Tenant lookup, OTP sign-in, refresh rotation, logout.

use crate::auth::tokens::{issue_pair, token_pair};
use crate::auth::AuthUser;
use crate::cache::keys;
use crate::crypto::{sha256_hex, verify_otp};
use crate::dto::*;
use crate::error::ApiError;
use crate::routes::Json;
use crate::services::{otp, views};
use crate::state::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::post;
use axum::Router;
use chrono::Utc;
use reelearn_db::repos::{auth as auth_repo, tenants, users};
use reelearn_domain::phone;
use serde::Deserialize;
use std::time::Duration;
use uuid::Uuid;

pub fn public_routes() -> Router<AppState> {
    Router::new()
        .route("/auth/tenant/lookup", post(tenant_lookup))
        .route("/auth/otp/request", post(otp_request))
        .route("/auth/otp/verify", post(otp_verify))
        .route("/auth/refresh", post(refresh))
}

pub fn private_routes() -> Router<AppState> {
    Router::new().route("/auth/logout", post(logout))
}

#[derive(Deserialize)]
struct LookupBody {
    slug: Option<String>,
    invite_token: Option<String>,
}

async fn tenant_lookup(
    State(state): State<AppState>,
    Json(body): Json<LookupBody>,
) -> Result<Json<TenantPublic>, ApiError> {
    // Invite links carry the slug before a dot: `reelearn://t/alwadi?inv=alwadi.<token>`.
    let slug = body
        .slug
        .filter(|s| !s.trim().is_empty())
        .or_else(|| {
            body.invite_token
                .as_deref()
                .and_then(|t| t.split('.').next())
                .map(|s| s.to_string())
        })
        .ok_or_else(|| ApiError::field("slug", "slug or invite_token is required"))?;
    let mut tx = state.db.admin_tx().await?;
    let tenant = tenants::find_by_slug(&mut tx, &slug.trim().to_ascii_lowercase())
        .await?
        .ok_or_else(|| ApiError::NotFound("no company space with that code".into()))?;
    Ok(Json(views::tenant_public_signed(&state, &tenant).await))
}

#[derive(Deserialize)]
struct OtpRequestBody {
    tenant_slug: String,
    phone: Option<String>,
    email: Option<String>,
}

#[derive(serde::Serialize)]
struct OtpRequested {
    challenge_id: Uuid,
    channel: String,
    expires_in_seconds: i64,
}

async fn otp_request(
    State(state): State<AppState>,
    Json(body): Json<OtpRequestBody>,
) -> Result<Json<OtpRequested>, ApiError> {
    let phone_e164 = match body.phone.as_deref().filter(|p| !p.trim().is_empty()) {
        Some(p) => Some(phone::normalize(p).map_err(|e| ApiError::field("phone", &e.to_string()))?),
        None => None,
    };
    let email = body
        .email
        .as_deref()
        .map(|e| e.trim().to_ascii_lowercase())
        .filter(|e| !e.is_empty());
    if phone_e164.is_none() && email.is_none() {
        return Err(ApiError::field("phone", "phone or email is required"));
    }
    let tenant = {
        let mut tx = state.db.admin_tx().await?;
        tenants::find_by_slug(&mut tx, &body.tenant_slug.trim().to_ascii_lowercase())
            .await?
            .ok_or_else(|| ApiError::NotFound("no company space with that code".into()))?
    };
    let mut tx = state.db.tenant_tx(tenant.id).await?;
    let user = match (&phone_e164, &email) {
        (Some(p), _) => users::find_by_phone(tx.conn(), p).await?,
        (None, Some(e)) => users::find_by_email(tx.conn(), e).await?,
        _ => None,
    };
    let user = user.ok_or_else(|| {
        ApiError::NotFound(
            "this number is not on your company's list; ask your manager to add you".into(),
        )
    })?;
    if user.status == reelearn_domain::UserStatus::Deactivated {
        return Err(ApiError::Forbidden(
            "this account has been deactivated".into(),
        ));
    }
    otp::check_request_rate(&state, &mut tx, phone_e164.as_deref(), email.as_deref()).await?;
    let challenge = otp::create(
        &state,
        &mut tx,
        Some(user.id),
        phone_e164.as_deref(),
        email.as_deref(),
    )
    .await?;
    tx.commit().await?;
    Ok(Json(OtpRequested {
        challenge_id: challenge.id,
        channel: challenge.channel,
        expires_in_seconds: challenge.expires_in_seconds,
    }))
}

#[derive(Deserialize)]
struct VerifyBody {
    challenge_id: Uuid,
    code: String,
    device: DeviceInfo,
}

async fn otp_verify(
    State(state): State<AppState>,
    Json(body): Json<VerifyBody>,
) -> Result<Json<TokenPair>, ApiError> {
    if body.code.len() < 4 || body.code.len() > 8 {
        return Err(ApiError::field("code", "must be 4 to 8 characters"));
    }
    let tenant_id = {
        let mut tx = state.db.admin_tx().await?;
        auth_repo::find_challenge_admin(&mut tx, body.challenge_id)
            .await?
            .map(|c| c.tenant_id)
            .ok_or_else(|| {
                ApiError::BadRequest("unknown or expired code; request a new one".into())
            })?
    };
    let mut tx = state.db.tenant_tx(tenant_id).await?;
    let challenge = auth_repo::find_challenge(tx.conn(), body.challenge_id)
        .await?
        .ok_or_else(|| ApiError::BadRequest("unknown or expired code; request a new one".into()))?;
    if challenge.consumed_at.is_some() {
        return Err(ApiError::BadRequest(
            "this code was already used; request a new one".into(),
        ));
    }
    if challenge.attempts >= state.config.otp_max_attempts {
        return Err(ApiError::Locked(
            "too many wrong attempts; request a new code in 15 minutes".into(),
        ));
    }
    if challenge.expires_at < Utc::now() {
        return Err(ApiError::BadRequest(
            "this code has expired; request a new one".into(),
        ));
    }
    if !verify_otp(body.code.trim(), &challenge.code_hash) {
        let attempts = auth_repo::bump_attempts(tx.conn(), challenge.id).await?;
        tx.commit().await?;
        if attempts >= state.config.otp_max_attempts {
            return Err(ApiError::Locked(
                "too many wrong attempts; request a new code in 15 minutes".into(),
            ));
        }
        return Err(ApiError::BadRequest(format!(
            "wrong code; {} attempts left",
            state.config.otp_max_attempts - attempts
        )));
    }
    auth_repo::consume_challenge(tx.conn(), challenge.id).await?;
    let user_id = challenge
        .user_id
        .ok_or_else(|| ApiError::BadRequest("this code is not linked to a person".into()))?;
    let tenant = tenants::get(tx.conn(), tenant_id).await?;
    let user_before = users::get(tx.conn(), user_id).await?;
    let me = views::me(&state, &mut tx, &user_before, &tenant).await?;
    users::upsert_device(
        tx.conn(),
        tenant_id,
        user_id,
        &body.device.device_id,
        &body.device.platform,
        body.device.app_version.as_deref(),
        body.device.push_token.as_deref(),
    )
    .await?;
    users::touch_last_seen(tx.conn(), user_id).await?;
    let user = users::get(tx.conn(), user_id).await?;
    let issued = issue_pair(&state, &mut tx, &user, Some(&body.device.device_id)).await?;
    tx.commit().await?;
    Ok(Json(token_pair(issued, me)))
}

#[derive(Deserialize)]
struct RefreshBody {
    refresh_token: String,
}

async fn refresh(
    State(state): State<AppState>,
    Json(body): Json<RefreshBody>,
) -> Result<Json<TokenPair>, ApiError> {
    let hash = sha256_hex(body.refresh_token.trim().as_bytes());
    let tenant_id = {
        let mut tx = state.db.admin_tx().await?;
        auth_repo::find_refresh_token_admin(&mut tx, &hash)
            .await?
            .map(|t| t.tenant_id)
            .ok_or_else(|| ApiError::Unauthorized("refresh token is not valid".into()))?
    };
    let mut tx = state.db.tenant_tx(tenant_id).await?;
    let row = auth_repo::find_refresh_token(tx.conn(), &hash)
        .await?
        .ok_or_else(|| ApiError::Unauthorized("refresh token is not valid".into()))?;
    if row.revoked_at.is_some() {
        // Reuse of a rotated token: someone replayed it. Revoke the whole family.
        auth_repo::revoke_all_for_user(tx.conn(), row.user_id).await?;
        tx.commit().await?;
        return Err(ApiError::Unauthorized(
            "refresh token was already used; sign in again".into(),
        ));
    }
    if row.expires_at < Utc::now() {
        return Err(ApiError::Unauthorized("refresh token has expired".into()));
    }
    let user = users::get(tx.conn(), row.user_id).await?;
    if user.status == reelearn_domain::UserStatus::Deactivated {
        return Err(ApiError::Unauthorized(
            "this account has been deactivated".into(),
        ));
    }
    let tenant = tenants::get(tx.conn(), tenant_id).await?;
    let issued = issue_pair(&state, &mut tx, &user, row.device_id.as_deref()).await?;
    let new_row =
        auth_repo::find_refresh_token(tx.conn(), &sha256_hex(issued.refresh_token.as_bytes()))
            .await?;
    auth_repo::rotate_refresh_token(tx.conn(), row.id, new_row.map(|r| r.id).unwrap_or(row.id))
        .await?;
    users::touch_last_seen(tx.conn(), user.id).await?;
    let me = views::me(&state, &mut tx, &user, &tenant).await?;
    tx.commit().await?;
    Ok(Json(token_pair(issued, me)))
}

#[derive(Deserialize, Default)]
struct LogoutBody {
    #[serde(default)]
    all_devices: bool,
    /// Extension: the refresh token of this session, so a single-device logout can revoke it.
    refresh_token: Option<String>,
}

async fn logout(
    State(state): State<AppState>,
    user: AuthUser,
    body: Option<Json<LogoutBody>>,
) -> Result<StatusCode, ApiError> {
    let body = body.map(|Json(b)| b).unwrap_or_default();
    let mut tx = state.db.tenant_tx(user.tenant_id).await?;
    if body.all_devices {
        auth_repo::revoke_all_for_user(tx.conn(), user.user_id).await?;
    } else if let Some(rt) = body.refresh_token.as_deref() {
        if let Some(row) =
            auth_repo::find_refresh_token(tx.conn(), &sha256_hex(rt.trim().as_bytes())).await?
        {
            if row.user_id == user.user_id {
                auth_repo::revoke_refresh_token(tx.conn(), row.id).await?;
            }
        }
    }
    tx.commit().await?;
    let remaining = (user.exp - Utc::now().timestamp()).max(1);
    state
        .cache
        .set_string(
            &keys::revoked(&user.jti),
            "1",
            Duration::from_secs(u64::try_from(remaining).unwrap_or(900)),
        )
        .await;
    Ok(StatusCode::NO_CONTENT)
}
