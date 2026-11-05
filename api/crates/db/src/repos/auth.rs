use crate::error::DbError;
use chrono::{DateTime, Utc};
use sqlx::PgConnection;
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct OtpChallengeRow {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub user_id: Option<Uuid>,
    pub phone_e164: Option<String>,
    pub email: Option<String>,
    pub channel: String,
    pub code_hash: String,
    pub attempts: i16,
    pub expires_at: DateTime<Utc>,
    pub consumed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

const OTP_COLS: &str = "id, tenant_id, user_id, phone_e164, email, channel, code_hash, attempts, expires_at, consumed_at, created_at";

pub async fn create_challenge(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    user_id: Option<Uuid>,
    phone: Option<&str>,
    email: Option<&str>,
    channel: &str,
    code_hash: &str,
    ttl_seconds: i64,
) -> Result<OtpChallengeRow, DbError> {
    Ok(sqlx::query_as::<_, OtpChallengeRow>(&format!(
        "INSERT INTO otp_challenges (tenant_id, user_id, phone_e164, email, channel, code_hash, expires_at)
         VALUES ($1, $2, $3, $4, $5, $6, now() + make_interval(secs => $7)) RETURNING {OTP_COLS}"
    ))
    .bind(tenant_id)
    .bind(user_id)
    .bind(phone)
    .bind(email)
    .bind(channel)
    .bind(code_hash)
    .bind(ttl_seconds as f64)
    .fetch_one(conn)
    .await?)
}

/// Cross-tenant: the verify call only carries the challenge id.
pub async fn find_challenge_admin(
    conn: &mut PgConnection,
    id: Uuid,
) -> Result<Option<OtpChallengeRow>, DbError> {
    Ok(sqlx::query_as::<_, OtpChallengeRow>(&format!(
        "SELECT {OTP_COLS} FROM otp_challenges WHERE id = $1"
    ))
    .bind(id)
    .fetch_optional(conn)
    .await?)
}

pub async fn find_challenge(
    conn: &mut PgConnection,
    id: Uuid,
) -> Result<Option<OtpChallengeRow>, DbError> {
    Ok(sqlx::query_as::<_, OtpChallengeRow>(&format!(
        "SELECT {OTP_COLS} FROM otp_challenges WHERE id = $1 FOR UPDATE"
    ))
    .bind(id)
    .fetch_optional(conn)
    .await?)
}

pub async fn bump_attempts(conn: &mut PgConnection, id: Uuid) -> Result<i16, DbError> {
    Ok(sqlx::query_scalar(
        "UPDATE otp_challenges SET attempts = attempts + 1 WHERE id = $1 RETURNING attempts",
    )
    .bind(id)
    .fetch_one(conn)
    .await?)
}

pub async fn consume_challenge(conn: &mut PgConnection, id: Uuid) -> Result<(), DbError> {
    sqlx::query("UPDATE otp_challenges SET consumed_at = now() WHERE id = $1")
        .bind(id)
        .execute(conn)
        .await?;
    Ok(())
}

/// Challenges requested for this phone in the last hour (DB-side throttle, backs up Redis).
pub async fn recent_challenge_count(
    conn: &mut PgConnection,
    phone: Option<&str>,
    email: Option<&str>,
) -> Result<i64, DbError> {
    Ok(sqlx::query_scalar(
        "SELECT count(*) FROM otp_challenges WHERE created_at > now() - interval '1 hour' AND ((phone_e164 IS NOT NULL AND phone_e164 = $1) OR (email IS NOT NULL AND lower(email) = lower($2)))",
    )
    .bind(phone)
    .bind(email)
    .fetch_one(conn)
    .await?)
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RefreshTokenRow {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub user_id: Uuid,
    pub device_id: Option<String>,
    pub token_hash: String,
    pub expires_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub replaced_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

const RT_COLS: &str = "id, tenant_id, user_id, device_id, token_hash, expires_at, revoked_at, replaced_by, created_at";

pub async fn insert_refresh_token(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    user_id: Uuid,
    device_id: Option<&str>,
    token_hash: &str,
    ttl_days: i64,
) -> Result<RefreshTokenRow, DbError> {
    Ok(sqlx::query_as::<_, RefreshTokenRow>(&format!(
        "INSERT INTO refresh_tokens (tenant_id, user_id, device_id, token_hash, expires_at)
         VALUES ($1, $2, $3, $4, now() + make_interval(days => $5)) RETURNING {RT_COLS}"
    ))
    .bind(tenant_id)
    .bind(user_id)
    .bind(device_id)
    .bind(token_hash)
    .bind(ttl_days as i32)
    .fetch_one(conn)
    .await?)
}

pub async fn find_refresh_token_admin(
    conn: &mut PgConnection,
    token_hash: &str,
) -> Result<Option<RefreshTokenRow>, DbError> {
    Ok(sqlx::query_as::<_, RefreshTokenRow>(&format!(
        "SELECT {RT_COLS} FROM refresh_tokens WHERE token_hash = $1"
    ))
    .bind(token_hash)
    .fetch_optional(conn)
    .await?)
}

pub async fn find_refresh_token(
    conn: &mut PgConnection,
    token_hash: &str,
) -> Result<Option<RefreshTokenRow>, DbError> {
    Ok(sqlx::query_as::<_, RefreshTokenRow>(&format!(
        "SELECT {RT_COLS} FROM refresh_tokens WHERE token_hash = $1 FOR UPDATE"
    ))
    .bind(token_hash)
    .fetch_optional(conn)
    .await?)
}

pub async fn rotate_refresh_token(
    conn: &mut PgConnection,
    old_id: Uuid,
    new_id: Uuid,
) -> Result<(), DbError> {
    sqlx::query("UPDATE refresh_tokens SET revoked_at = now(), replaced_by = $2 WHERE id = $1")
        .bind(old_id)
        .bind(new_id)
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn revoke_refresh_token(conn: &mut PgConnection, id: Uuid) -> Result<(), DbError> {
    sqlx::query(
        "UPDATE refresh_tokens SET revoked_at = now() WHERE id = $1 AND revoked_at IS NULL",
    )
    .bind(id)
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn revoke_all_for_user(conn: &mut PgConnection, user_id: Uuid) -> Result<u64, DbError> {
    Ok(sqlx::query(
        "UPDATE refresh_tokens SET revoked_at = now() WHERE user_id = $1 AND revoked_at IS NULL",
    )
    .bind(user_id)
    .execute(conn)
    .await?
    .rows_affected())
}

pub async fn revoke_for_device(
    conn: &mut PgConnection,
    user_id: Uuid,
    device_id: &str,
) -> Result<u64, DbError> {
    Ok(sqlx::query("UPDATE refresh_tokens SET revoked_at = now() WHERE user_id = $1 AND device_id = $2 AND revoked_at IS NULL")
        .bind(user_id)
        .bind(device_id)
        .execute(conn)
        .await?
        .rows_affected())
}
