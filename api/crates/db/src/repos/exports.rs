use crate::error::DbError;
use chrono::{DateTime, NaiveDate, Utc};
use reelearn_domain::{ExportKind, JobStatus};
use sqlx::PgConnection;
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ExportRow {
    pub id: Uuid,
    pub kind: ExportKind,
    pub requested_by: Uuid,
    pub subject_user_id: Option<Uuid>,
    pub period_start: Option<NaiveDate>,
    pub period_end: Option<NaiveDate>,
    pub status: JobStatus,
    pub file_key: Option<String>,
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
}

const COLS: &str = "id, kind, requested_by, subject_user_id, period_start, period_end, status, file_key, error, created_at, finished_at";

pub async fn create(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    kind: ExportKind,
    requested_by: Uuid,
    subject_user_id: Option<Uuid>,
    period_start: Option<NaiveDate>,
    period_end: Option<NaiveDate>,
) -> Result<ExportRow, DbError> {
    Ok(sqlx::query_as::<_, ExportRow>(&format!(
        "INSERT INTO exports (tenant_id, kind, requested_by, subject_user_id, period_start, period_end) VALUES ($1, $2, $3, $4, $5, $6) RETURNING {COLS}"
    ))
    .bind(tenant_id)
    .bind(kind)
    .bind(requested_by)
    .bind(subject_user_id)
    .bind(period_start)
    .bind(period_end)
    .fetch_one(conn)
    .await?)
}

pub async fn list(conn: &mut PgConnection) -> Result<Vec<ExportRow>, DbError> {
    Ok(sqlx::query_as::<_, ExportRow>(&format!(
        "SELECT {COLS} FROM exports ORDER BY created_at DESC LIMIT 100"
    ))
    .fetch_all(conn)
    .await?)
}

pub async fn get(conn: &mut PgConnection, id: Uuid) -> Result<ExportRow, DbError> {
    sqlx::query_as::<_, ExportRow>(&format!("SELECT {COLS} FROM exports WHERE id = $1"))
        .bind(id)
        .fetch_optional(conn)
        .await?
        .ok_or(DbError::NotFound("export"))
}

pub async fn finish(
    conn: &mut PgConnection,
    id: Uuid,
    status: JobStatus,
    file_key: Option<&str>,
    error: Option<&str>,
) -> Result<(), DbError> {
    sqlx::query("UPDATE exports SET status = $2, file_key = coalesce($3, file_key), error = $4, finished_at = now() WHERE id = $1")
        .bind(id)
        .bind(status)
        .bind(file_key)
        .bind(error)
        .execute(conn)
        .await?;
    Ok(())
}
