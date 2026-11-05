use crate::error::DbError;
use chrono::{DateTime, Utc};
use sqlx::PgConnection;
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct MediaAssetRow {
    pub id: Uuid,
    pub storage_key: String,
    pub mime: String,
    pub bytes: i64,
    pub sha256: Option<String>,
    pub duration_ms: Option<i32>,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

const COLS: &str = "id, storage_key, mime, bytes, sha256, duration_ms, created_by, created_at";

pub async fn create(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    storage_key: &str,
    mime: &str,
    bytes: i64,
    sha256: Option<&str>,
    created_by: Option<Uuid>,
) -> Result<MediaAssetRow, DbError> {
    Ok(sqlx::query_as::<_, MediaAssetRow>(&format!(
        "INSERT INTO media_assets (tenant_id, storage_key, mime, bytes, sha256, created_by) VALUES ($1, $2, $3, $4, $5, $6) RETURNING {COLS}"
    ))
    .bind(tenant_id)
    .bind(storage_key)
    .bind(mime)
    .bind(bytes)
    .bind(sha256)
    .bind(created_by)
    .fetch_one(conn)
    .await?)
}

pub async fn get(conn: &mut PgConnection, id: Uuid) -> Result<MediaAssetRow, DbError> {
    sqlx::query_as::<_, MediaAssetRow>(&format!("SELECT {COLS} FROM media_assets WHERE id = $1"))
        .bind(id)
        .fetch_optional(conn)
        .await?
        .ok_or(DbError::NotFound("media asset"))
}

pub async fn find_by_sha(
    conn: &mut PgConnection,
    sha256: &str,
) -> Result<Option<MediaAssetRow>, DbError> {
    Ok(sqlx::query_as::<_, MediaAssetRow>(&format!(
        "SELECT {COLS} FROM media_assets WHERE sha256 = $1 LIMIT 1"
    ))
    .bind(sha256)
    .fetch_optional(conn)
    .await?)
}
