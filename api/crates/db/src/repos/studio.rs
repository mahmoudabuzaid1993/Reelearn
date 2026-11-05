//! Sources, extractions, moments, generation jobs and steps.

use crate::error::DbError;
use crate::repos::push_cursor_filter;
use chrono::{DateTime, Utc};
use reelearn_domain::cursor::Cursor;
use reelearn_domain::{ExtractionStatus, GenStep, JobStatus, SourceKind};
use sqlx::{PgConnection, QueryBuilder};
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SourceRow {
    pub id: Uuid,
    pub kind: SourceKind,
    pub title: String,
    pub url: Option<String>,
    pub media_asset_id: Option<Uuid>,
    pub prompt: Option<String>,
    pub language: Option<String>,
    pub extraction_status: ExtractionStatus,
    pub extraction_error: Option<String>,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

const SCOLS: &str = "id, kind, title, url, media_asset_id, prompt, language, extraction_status, extraction_error, created_by, created_at, updated_at";

pub async fn get_source(conn: &mut PgConnection, id: Uuid) -> Result<SourceRow, DbError> {
    sqlx::query_as::<_, SourceRow>(&format!("SELECT {SCOLS} FROM sources WHERE id = $1"))
        .bind(id)
        .fetch_optional(conn)
        .await?
        .ok_or(DbError::NotFound("source"))
}

pub async fn list_sources(
    conn: &mut PgConnection,
    cursor: Option<Cursor>,
    limit: i64,
) -> Result<Vec<SourceRow>, DbError> {
    let mut qb: QueryBuilder<sqlx::Postgres> =
        QueryBuilder::new(format!("SELECT {SCOLS} FROM sources s WHERE true"));
    push_cursor_filter(&mut qb, "s.created_at", "s.id", cursor);
    qb.push(" ORDER BY s.created_at DESC, s.id DESC LIMIT ");
    qb.push_bind(limit + 1);
    Ok(qb.build_query_as::<SourceRow>().fetch_all(conn).await?)
}

pub async fn create_source(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    kind: SourceKind,
    title: &str,
    url: Option<&str>,
    media_asset_id: Option<Uuid>,
    prompt: Option<&str>,
    language: Option<&str>,
    created_by: Uuid,
) -> Result<SourceRow, DbError> {
    Ok(sqlx::query_as::<_, SourceRow>(&format!(
        "INSERT INTO sources (tenant_id, kind, title, url, media_asset_id, prompt, language, created_by) VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING {SCOLS}"
    ))
    .bind(tenant_id)
    .bind(kind)
    .bind(title)
    .bind(url)
    .bind(media_asset_id)
    .bind(prompt)
    .bind(language)
    .bind(created_by)
    .fetch_one(conn)
    .await?)
}

pub async fn set_extraction_status(
    conn: &mut PgConnection,
    id: Uuid,
    status: ExtractionStatus,
    error: Option<&str>,
) -> Result<(), DbError> {
    sqlx::query("UPDATE sources SET extraction_status = $2, extraction_error = $3 WHERE id = $1")
        .bind(id)
        .bind(status)
        .bind(error)
        .execute(conn)
        .await?;
    Ok(())
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ExtractionRow {
    pub source_id: Uuid,
    pub text: String,
    pub summary: serde_json::Value,
    pub language: Option<String>,
    pub word_count: Option<i32>,
    pub extracted_at: DateTime<Utc>,
}

pub async fn extraction(
    conn: &mut PgConnection,
    source_id: Uuid,
) -> Result<Option<ExtractionRow>, DbError> {
    Ok(sqlx::query_as::<_, ExtractionRow>("SELECT source_id, text, summary, language, word_count, extracted_at FROM source_extractions WHERE source_id = $1")
        .bind(source_id)
        .fetch_optional(conn)
        .await?)
}

pub async fn upsert_extraction(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    source_id: Uuid,
    text: &str,
    summary: &serde_json::Value,
    language: Option<&str>,
) -> Result<(), DbError> {
    let words = i32::try_from(text.split_whitespace().count()).unwrap_or(i32::MAX);
    sqlx::query(
        "INSERT INTO source_extractions (source_id, tenant_id, text, summary, language, word_count) VALUES ($1, $2, $3, $4, $5, $6)
         ON CONFLICT (source_id) DO UPDATE SET text = EXCLUDED.text, summary = EXCLUDED.summary, language = EXCLUDED.language, word_count = EXCLUDED.word_count, extracted_at = now()",
    )
    .bind(source_id)
    .bind(tenant_id)
    .bind(text)
    .bind(summary)
    .bind(language)
    .bind(words)
    .execute(conn)
    .await?;
    Ok(())
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct MomentRow {
    pub id: Uuid,
    pub source_id: Uuid,
    pub start_ms: i32,
    pub end_ms: i32,
    pub excerpt: String,
    pub title: Option<String>,
    pub score: f32,
    pub speaker: Option<String>,
    pub selected: bool,
    pub reel_id: Option<Uuid>,
}

const MCOLS: &str =
    "id, source_id, start_ms, end_ms, excerpt, title, score, speaker, selected, reel_id";

pub async fn moments(conn: &mut PgConnection, source_id: Uuid) -> Result<Vec<MomentRow>, DbError> {
    Ok(sqlx::query_as::<_, MomentRow>(&format!(
        "SELECT {MCOLS} FROM source_moments WHERE source_id = $1 ORDER BY score DESC, start_ms"
    ))
    .bind(source_id)
    .fetch_all(conn)
    .await?)
}

pub async fn moment(conn: &mut PgConnection, id: Uuid) -> Result<MomentRow, DbError> {
    sqlx::query_as::<_, MomentRow>(&format!("SELECT {MCOLS} FROM source_moments WHERE id = $1"))
        .bind(id)
        .fetch_optional(conn)
        .await?
        .ok_or(DbError::NotFound("moment"))
}

pub async fn insert_moment(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    source_id: Uuid,
    start_ms: i32,
    end_ms: i32,
    excerpt: &str,
    title: Option<&str>,
    score: f32,
    speaker: Option<&str>,
) -> Result<MomentRow, DbError> {
    Ok(sqlx::query_as::<_, MomentRow>(&format!(
        "INSERT INTO source_moments (tenant_id, source_id, start_ms, end_ms, excerpt, title, score, speaker) VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING {MCOLS}"
    ))
    .bind(tenant_id)
    .bind(source_id)
    .bind(start_ms)
    .bind(end_ms)
    .bind(excerpt)
    .bind(title)
    .bind(score)
    .bind(speaker)
    .fetch_one(conn)
    .await?)
}

pub async fn delete_moments(conn: &mut PgConnection, source_id: Uuid) -> Result<(), DbError> {
    sqlx::query("DELETE FROM source_moments WHERE source_id = $1 AND reel_id IS NULL")
        .bind(source_id)
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn mark_moment_cut(
    conn: &mut PgConnection,
    id: Uuid,
    reel_id: Uuid,
    start_ms: i32,
    end_ms: i32,
    title: Option<&str>,
) -> Result<(), DbError> {
    sqlx::query("UPDATE source_moments SET selected = true, reel_id = $2, start_ms = $3, end_ms = $4, title = coalesce($5, title) WHERE id = $1")
        .bind(id)
        .bind(reel_id)
        .bind(start_ms)
        .bind(end_ms)
        .bind(title)
        .execute(conn)
        .await?;
    Ok(())
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct GenerationJobRow {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub source_id: Uuid,
    pub requested_by: Uuid,
    pub reviewer_id: Option<Uuid>,
    pub params: serde_json::Value,
    pub status: JobStatus,
    pub progress_pct: i16,
    pub cost_credits: i32,
    pub error: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl GenerationJobRow {
    pub fn param_str(&self, key: &str) -> Option<String> {
        self.params
            .get(key)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }
    pub fn param_uuid(&self, key: &str) -> Option<Uuid> {
        self.param_str(key).and_then(|s| s.parse().ok())
    }
    pub fn param_languages(&self) -> Vec<String> {
        self.params
            .get("languages")
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default()
    }
}

const JCOLS: &str = "id, tenant_id, source_id, requested_by, reviewer_id, params, status, progress_pct, cost_credits, error, started_at, finished_at, created_at, updated_at";

pub async fn get_job(conn: &mut PgConnection, id: Uuid) -> Result<GenerationJobRow, DbError> {
    sqlx::query_as::<_, GenerationJobRow>(&format!(
        "SELECT {JCOLS} FROM generation_jobs WHERE id = $1"
    ))
    .bind(id)
    .fetch_optional(conn)
    .await?
    .ok_or(DbError::NotFound("generation job"))
}

pub async fn list_jobs(
    conn: &mut PgConnection,
    cursor: Option<Cursor>,
    limit: i64,
) -> Result<Vec<GenerationJobRow>, DbError> {
    let mut qb: QueryBuilder<sqlx::Postgres> =
        QueryBuilder::new(format!("SELECT {JCOLS} FROM generation_jobs j WHERE true"));
    push_cursor_filter(&mut qb, "j.created_at", "j.id", cursor);
    qb.push(" ORDER BY j.created_at DESC, j.id DESC LIMIT ");
    qb.push_bind(limit + 1);
    Ok(qb
        .build_query_as::<GenerationJobRow>()
        .fetch_all(conn)
        .await?)
}

pub async fn create_job(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    source_id: Uuid,
    requested_by: Uuid,
    reviewer_id: Option<Uuid>,
    params: &serde_json::Value,
    cost_credits: i32,
) -> Result<GenerationJobRow, DbError> {
    Ok(sqlx::query_as::<_, GenerationJobRow>(&format!(
        "INSERT INTO generation_jobs (tenant_id, source_id, requested_by, reviewer_id, params, cost_credits) VALUES ($1, $2, $3, $4, $5, $6) RETURNING {JCOLS}"
    ))
    .bind(tenant_id)
    .bind(source_id)
    .bind(requested_by)
    .bind(reviewer_id)
    .bind(params)
    .bind(cost_credits)
    .fetch_one(conn)
    .await?)
}

pub async fn set_job_status(
    conn: &mut PgConnection,
    id: Uuid,
    status: JobStatus,
    progress_pct: Option<i16>,
    error: Option<&str>,
) -> Result<(), DbError> {
    sqlx::query(
        "UPDATE generation_jobs SET status = $2, progress_pct = coalesce($3, progress_pct), error = $4,
           started_at = CASE WHEN $2 = 'running'::job_status_t THEN coalesce(started_at, now()) ELSE started_at END,
           finished_at = CASE WHEN $2 IN ('done'::job_status_t, 'failed'::job_status_t, 'cancelled'::job_status_t) THEN now() ELSE NULL END
         WHERE id = $1",
    )
    .bind(id)
    .bind(status)
    .bind(progress_pct)
    .bind(error)
    .execute(conn)
    .await?;
    Ok(())
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct StepRow {
    pub id: Uuid,
    pub job_id: Uuid,
    pub position: i16,
    pub step: GenStep,
    pub status: JobStatus,
    pub provider: Option<String>,
    pub provider_ref: Option<String>,
    pub attempt: i16,
    pub log: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
}

const STCOLS: &str = "id, job_id, position, step, status, provider, provider_ref, attempt, log, started_at, finished_at";

pub async fn steps(conn: &mut PgConnection, job_id: Uuid) -> Result<Vec<StepRow>, DbError> {
    Ok(sqlx::query_as::<_, StepRow>(&format!(
        "SELECT {STCOLS} FROM generation_steps WHERE job_id = $1 ORDER BY position"
    ))
    .bind(job_id)
    .fetch_all(conn)
    .await?)
}

pub async fn step_at(
    conn: &mut PgConnection,
    job_id: Uuid,
    position: i16,
) -> Result<Option<StepRow>, DbError> {
    Ok(sqlx::query_as::<_, StepRow>(&format!(
        "SELECT {STCOLS} FROM generation_steps WHERE job_id = $1 AND position = $2 FOR UPDATE"
    ))
    .bind(job_id)
    .bind(position)
    .fetch_optional(conn)
    .await?)
}

pub async fn insert_steps(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    job_id: Uuid,
    steps: &[GenStep],
) -> Result<(), DbError> {
    for (i, s) in steps.iter().enumerate() {
        sqlx::query("INSERT INTO generation_steps (tenant_id, job_id, position, step) VALUES ($1, $2, $3, $4)")
            .bind(tenant_id)
            .bind(job_id)
            .bind(i16::try_from(i).unwrap_or(i16::MAX))
            .bind(s)
            .execute(&mut *conn)
            .await?;
    }
    Ok(())
}

pub async fn update_step(
    conn: &mut PgConnection,
    id: Uuid,
    status: JobStatus,
    provider: Option<&str>,
    provider_ref: Option<&str>,
    log: Option<&str>,
    bump_attempt: bool,
) -> Result<(), DbError> {
    sqlx::query(
        "UPDATE generation_steps SET status = $2, provider = coalesce($3, provider), provider_ref = coalesce($4, provider_ref), log = coalesce($5, log),
           attempt = attempt + CASE WHEN $6 THEN 1 ELSE 0 END,
           started_at = CASE WHEN $2 = 'running'::job_status_t THEN now() ELSE started_at END,
           finished_at = CASE WHEN $2 IN ('done'::job_status_t, 'failed'::job_status_t) THEN now() ELSE finished_at END
         WHERE id = $1",
    )
    .bind(id)
    .bind(status)
    .bind(provider)
    .bind(provider_ref)
    .bind(log)
    .bind(bump_attempt)
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn step_by_provider_ref(
    conn: &mut PgConnection,
    provider_ref: &str,
) -> Result<Option<StepRow>, DbError> {
    Ok(sqlx::query_as::<_, StepRow>(&format!(
        "SELECT {STCOLS} FROM generation_steps WHERE provider_ref = $1 LIMIT 1"
    ))
    .bind(provider_ref)
    .fetch_optional(conn)
    .await?)
}

/// A source whose transcription is parked on a provider reference (`summary.pending_ref`).
/// Cross-tenant: used by the provider callback before it knows the tenant.
pub async fn source_by_pending_ref(
    conn: &mut PgConnection,
    provider_ref: &str,
) -> Result<Option<(Uuid, Uuid)>, DbError> {
    Ok(sqlx::query_as::<_, (Uuid, Uuid)>("SELECT e.tenant_id, e.source_id FROM source_extractions e WHERE e.summary->>'pending_ref' = $1 LIMIT 1")
        .bind(provider_ref)
        .fetch_optional(conn)
        .await?)
}

/// The most recent generation job whose `params[key]` equals `value` (e.g. the cut job of a reel).
pub async fn find_job_by_param(
    conn: &mut PgConnection,
    key: &str,
    value: &str,
) -> Result<Option<GenerationJobRow>, DbError> {
    Ok(sqlx::query_as::<_, GenerationJobRow>(&format!("SELECT {JCOLS} FROM generation_jobs WHERE params->>$1 = $2 ORDER BY created_at DESC LIMIT 1"))
        .bind(key)
        .bind(value)
        .fetch_optional(conn)
        .await?)
}
