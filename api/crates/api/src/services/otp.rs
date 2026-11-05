//! OTP challenges: dev code 1234 (logged, never sent); otherwise a random 6-digit code
//! queued as a `send_delivery` job so a request handler never calls a provider.

use crate::cache::keys;
use crate::crypto::{hash_otp, random_otp_code};
use crate::error::ApiError;
use crate::state::AppState;
use reelearn_db::repos::{auth as auth_repo, jobs};
use reelearn_db::TenantTx;
use std::time::Duration;
use uuid::Uuid;

pub const DEV_CODE: &str = "1234";

pub struct Challenge {
    pub id: Uuid,
    pub channel: String,
    pub expires_in_seconds: i64,
}

/// Rate limit: `OTP_REQUESTS_PER_HOUR` per phone/email, Redis first, DB as the backstop.
pub async fn check_request_rate(
    state: &AppState,
    tx: &mut TenantTx,
    phone: Option<&str>,
    email: Option<&str>,
) -> Result<(), ApiError> {
    let id = phone.or(email).unwrap_or("?").to_ascii_lowercase();
    let limit = state.config.otp_requests_per_hour;
    match state
        .cache
        .incr_window(&keys::rate_limit("otp", &id), Duration::from_secs(3600))
        .await
    {
        Some(n) if n > limit => {
            let retry = state
                .cache
                .ttl_seconds(&keys::rate_limit("otp", &id))
                .await
                .unwrap_or(3600);
            return Err(ApiError::TooManyRequests {
                retry_after_seconds: u64::try_from(retry).unwrap_or(3600),
            });
        }
        Some(_) => {}
        None => {
            let recent = auth_repo::recent_challenge_count(tx.conn(), phone, email).await?;
            if recent >= limit {
                return Err(ApiError::TooManyRequests {
                    retry_after_seconds: 3600,
                });
            }
        }
    }
    Ok(())
}

pub async fn create(
    state: &AppState,
    tx: &mut TenantTx,
    user_id: Option<Uuid>,
    phone: Option<&str>,
    email: Option<&str>,
) -> Result<Challenge, ApiError> {
    let tenant_id = tx.tenant_id();
    let channel = if phone.is_some() { "whatsapp" } else { "email" };
    let code = if state.is_development() {
        DEV_CODE.to_string()
    } else {
        random_otp_code()
    };
    let hash = hash_otp(&code).map_err(ApiError::internal)?;
    let row = auth_repo::create_challenge(
        tx.conn(),
        tenant_id,
        user_id,
        phone,
        email,
        channel,
        &hash,
        state.config.otp_ttl_seconds,
    )
    .await?;
    if state.is_development() {
        tracing::info!(challenge_id = %row.id, to = phone.or(email).unwrap_or("?"), code = %code, "development OTP (not sent)");
    } else {
        jobs::enqueue(
            tx.conn(),
            Some(tenant_id),
            "send_delivery",
            serde_json::json!({ "kind": "otp", "channel": channel, "to": phone.or(email), "code": code, "challenge_id": row.id }),
            None,
        )
        .await?;
    }
    Ok(Challenge {
        id: row.id,
        channel: channel.to_string(),
        expires_in_seconds: state.config.otp_ttl_seconds,
    })
}
