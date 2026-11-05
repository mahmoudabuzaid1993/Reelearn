use crate::error::DbError;
use chrono::{DateTime, Utc};
use reelearn_domain::IntegrationKind;
use sqlx::PgConnection;
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct IntegrationRow {
    pub id: Uuid,
    pub kind: IntegrationKind,
    pub status: String,
    pub config_enc: Option<Vec<u8>>,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub updated_at: DateTime<Utc>,
}

const COLS: &str = "id, kind, status, config_enc, last_sync_at, last_error, updated_at";

pub async fn list(conn: &mut PgConnection) -> Result<Vec<IntegrationRow>, DbError> {
    Ok(sqlx::query_as::<_, IntegrationRow>(&format!(
        "SELECT {COLS} FROM integrations ORDER BY kind"
    ))
    .fetch_all(conn)
    .await?)
}

pub async fn find(
    conn: &mut PgConnection,
    kind: IntegrationKind,
) -> Result<Option<IntegrationRow>, DbError> {
    Ok(sqlx::query_as::<_, IntegrationRow>(&format!(
        "SELECT {COLS} FROM integrations WHERE kind = $1"
    ))
    .bind(kind)
    .fetch_optional(conn)
    .await?)
}

pub async fn upsert(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    kind: IntegrationKind,
    status: &str,
    config_enc: &[u8],
) -> Result<IntegrationRow, DbError> {
    Ok(sqlx::query_as::<_, IntegrationRow>(&format!(
        "INSERT INTO integrations (tenant_id, kind, status, config_enc) VALUES ($1, $2, $3, $4)
         ON CONFLICT (tenant_id, kind) DO UPDATE SET status = EXCLUDED.status, config_enc = EXCLUDED.config_enc, last_error = NULL RETURNING {COLS}"
    ))
    .bind(tenant_id)
    .bind(kind)
    .bind(status)
    .bind(config_enc)
    .fetch_one(conn)
    .await?)
}

pub async fn set_status(
    conn: &mut PgConnection,
    kind: IntegrationKind,
    status: &str,
    error: Option<&str>,
) -> Result<(), DbError> {
    sqlx::query("UPDATE integrations SET status = $2, last_error = $3, last_sync_at = CASE WHEN $2 = 'connected' THEN now() ELSE last_sync_at END WHERE kind = $1")
        .bind(kind)
        .bind(status)
        .bind(error)
        .execute(conn)
        .await?;
    Ok(())
}
