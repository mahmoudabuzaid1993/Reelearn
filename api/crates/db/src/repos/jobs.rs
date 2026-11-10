//! The durable job queue (`jobs` table). Popped with `FOR UPDATE SKIP LOCKED`.
//! The queue is not tenant-scoped: it is read with the admin connection; each job
//! carries the `tenant_id` the worker opens a [`crate::TenantTx`] for.

use crate::error::DbError;
use chrono::{DateTime, NaiveDate, Utc};
use reelearn_domain::JobStatus;
use sqlx::PgConnection;
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct JobRow {
    pub id: Uuid,
    pub tenant_id: Option<Uuid>,
    pub kind: String,
    pub payload: serde_json::Value,
    pub status: JobStatus,
    pub run_at: DateTime<Utc>,
    pub attempts: i16,
    pub max_attempts: i16,
    pub locked_by: Option<String>,
    pub locked_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
}

const COLS: &str = "id, tenant_id, kind, payload, status, run_at, attempts, max_attempts, locked_by, locked_at, last_error, created_at, finished_at";

pub async fn enqueue(
    conn: &mut PgConnection,
    tenant_id: Option<Uuid>,
    kind: &str,
    payload: serde_json::Value,
    run_at: Option<DateTime<Utc>>,
) -> Result<JobRow, DbError> {
    Ok(sqlx::query_as::<_, JobRow>(&format!(
        "INSERT INTO jobs (tenant_id, kind, payload, run_at) VALUES ($1, $2, $3, coalesce($4, now())) RETURNING {COLS}"
    ))
    .bind(tenant_id)
    .bind(kind)
    .bind(payload)
    .bind(run_at)
    .fetch_one(conn)
    .await?)
}

/// Claim the next due job (oldest `run_at` first), skipping rows other workers hold.
pub async fn claim(
    conn: &mut PgConnection,
    worker_id: &str,
    kinds: Option<&[String]>,
) -> Result<Option<JobRow>, DbError> {
    let sql = format!(
        "UPDATE jobs SET status = 'running', locked_by = $1, locked_at = now(), attempts = attempts + 1
         WHERE id = (
           SELECT id FROM jobs WHERE status = 'queued' AND run_at <= now() AND ($2::text[] IS NULL OR kind = ANY($2))
           ORDER BY run_at, created_at FOR UPDATE SKIP LOCKED LIMIT 1
         ) RETURNING {COLS}"
    );
    Ok(sqlx::query_as::<_, JobRow>(&sql)
        .bind(worker_id)
        .bind(kinds)
        .fetch_optional(conn)
        .await?)
}

pub async fn complete(conn: &mut PgConnection, id: Uuid) -> Result<(), DbError> {
    sqlx::query("UPDATE jobs SET status = 'done', finished_at = now(), locked_by = NULL, locked_at = NULL, last_error = NULL WHERE id = $1")
        .bind(id)
        .execute(conn)
        .await?;
    Ok(())
}

/// Re-queue with a delay, or dead-letter once `max_attempts` is reached.
pub async fn fail(
    conn: &mut PgConnection,
    id: Uuid,
    error: &str,
    retry_at: Option<DateTime<Utc>>,
) -> Result<JobStatus, DbError> {
    let status: JobStatus = sqlx::query_scalar(
        "UPDATE jobs SET
           status = CASE WHEN attempts >= max_attempts OR $3::timestamptz IS NULL THEN 'failed'::job_status_t ELSE 'queued'::job_status_t END,
           run_at = coalesce($3, run_at),
           last_error = left($2, 2000),
           locked_by = NULL, locked_at = NULL,
           finished_at = CASE WHEN attempts >= max_attempts OR $3::timestamptz IS NULL THEN now() ELSE NULL END
         WHERE id = $1 RETURNING status",
    )
    .bind(id)
    .bind(error)
    .bind(retry_at)
    .fetch_one(conn)
    .await?;
    Ok(status)
}

/// Jobs stuck in `running` for longer than `minutes` (worker died): back to the queue.
pub async fn release_stale(conn: &mut PgConnection, minutes: i32) -> Result<u64, DbError> {
    Ok(sqlx::query("UPDATE jobs SET status = 'queued', locked_by = NULL, locked_at = NULL WHERE status = 'running' AND locked_at < now() - make_interval(mins => $1)")
        .bind(minutes)
        .execute(conn)
        .await?
        .rows_affected())
}

pub async fn exists_for_day(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    kind: &str,
    day: NaiveDate,
) -> Result<bool, DbError> {
    Ok(sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM jobs WHERE tenant_id = $1 AND kind = $2 AND payload->>'day' = $3 AND status <> 'failed')")
        .bind(tenant_id)
        .bind(kind)
        .bind(day.to_string())
        .fetch_one(conn)
        .await?)
}

pub async fn get(conn: &mut PgConnection, id: Uuid) -> Result<Option<JobRow>, DbError> {
    Ok(
        sqlx::query_as::<_, JobRow>(&format!("SELECT {COLS} FROM jobs WHERE id = $1"))
            .bind(id)
            .fetch_optional(conn)
            .await?,
    )
}

pub async fn count_by_status(conn: &mut PgConnection, status: JobStatus) -> Result<i64, DbError> {
    Ok(
        sqlx::query_scalar("SELECT count(*) FROM jobs WHERE status = $1")
            .bind(status)
            .fetch_one(conn)
            .await?,
    )
}

/// Is there a queued/running job for this generation job already (avoids double-enqueue on retry)?
pub async fn active_for_generation(
    conn: &mut PgConnection,
    generation_job_id: Uuid,
) -> Result<bool, DbError> {
    Ok(sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM jobs WHERE payload->>'generation_job_id' = $1 AND status IN ('queued','running'))")
        .bind(generation_job_id.to_string())
        .fetch_one(conn)
        .await?)
}

/// Is a step job for this generation job and position already queued or running?
pub async fn active_for_generation_position(
    conn: &mut PgConnection,
    generation_job_id: Uuid,
    position: i16,
) -> Result<bool, DbError> {
    Ok(sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM jobs WHERE payload->>'generation_job_id' = $1 AND (payload->>'position')::int = $2 AND status IN ('queued','running'))")
        .bind(generation_job_id.to_string())
        .bind(i32::from(position))
        .fetch_one(conn)
        .await?)
}

/// Is a send job for this delivery already queued or running? (The scheduler fan-out must not double-send.)
pub async fn active_for_delivery(
    conn: &mut PgConnection,
    delivery_id: Uuid,
) -> Result<bool, DbError> {
    Ok(sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM jobs WHERE payload->>'delivery_id' = $1 AND status IN ('queued','running'))")
        .bind(delivery_id.to_string())
        .fetch_one(conn)
        .await?)
}

/// A job of `kind` for the tenant whose payload carries `key = value` (hourly stamps).
pub async fn exists_with_key(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    kind: &str,
    key: &str,
    value: &str,
) -> Result<bool, DbError> {
    Ok(sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM jobs WHERE tenant_id = $1 AND kind = $2 AND payload->>$3 = $4 AND status <> 'failed')")
        .bind(tenant_id)
        .bind(kind)
        .bind(key)
        .bind(value)
        .fetch_one(conn)
        .await?)
}
