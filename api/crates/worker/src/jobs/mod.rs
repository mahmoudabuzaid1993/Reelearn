//! Job kinds and their handlers. `dispatch` is the only place that maps `jobs.kind` to code.

pub mod deliveries;
pub mod engine;
pub mod exports;
pub mod pipeline;
pub mod plans;

use crate::Ctx;
use reelearn_db::repos::jobs::JobRow;
use reelearn_db::DbError;

#[derive(Debug, thiserror::Error)]
pub enum JobError {
    /// Retried with backoff until `max_attempts`.
    #[error("{0}")]
    Transient(String),
    /// Dead-lettered immediately; the message is shown to the admin.
    #[error("{0}")]
    Permanent(String),
}

impl From<DbError> for JobError {
    fn from(e: DbError) -> Self {
        match e {
            DbError::NotFound(_)
            | DbError::Invalid(_)
            | DbError::Conflict(_)
            | DbError::DualApprovalRequired
            | DbError::InsufficientCredits { .. } => JobError::Permanent(e.to_string()),
            other => JobError::Transient(other.to_string()),
        }
    }
}

impl From<sqlx::Error> for JobError {
    fn from(e: sqlx::Error) -> Self {
        JobError::Transient(e.to_string())
    }
}

#[derive(Debug)]
pub struct JobOutcome {
    pub note: String,
}

impl JobOutcome {
    pub fn done(note: String) -> JobOutcome {
        JobOutcome { note }
    }
}

pub const KINDS: &[&str] = &[
    "extract_source",
    "script_reels",
    "write_question",
    "render_reel",
    "voice_reel",
    "caption_reel",
    "translate_reel",
    "transcribe_podcast",
    "find_moments",
    "cut_moment",
    "provider_callback",
    "build_daily_plans",
    "send_delivery",
    "hrdf_export",
    "data_export",
    "maintain_partitions",
    "engine_hourly",
];

pub async fn dispatch(ctx: &Ctx, job: &JobRow) -> Result<JobOutcome, JobError> {
    let tenant = || {
        job.tenant_id
            .ok_or_else(|| JobError::Permanent(format!("{} job without tenant_id", job.kind)))
    };
    // A job that carries a generation_job_id is a pipeline step; the same kinds without one are source-only jobs.
    let is_step = job.payload.get("generation_job_id").is_some();
    match job.kind.as_str() {
        "extract_source" if is_step => pipeline::run_step(ctx, tenant()?, &job.payload).await,
        "extract_source" => pipeline::extract_source(ctx, tenant()?, &job.payload).await,
        "transcribe_podcast" => pipeline::transcribe_podcast(ctx, tenant()?, &job.payload).await,
        "find_moments" => pipeline::find_moments(ctx, tenant()?, &job.payload).await,
        "cut_moment" => pipeline::cut_moment(ctx, tenant()?, &job.payload).await,
        "script_reels" | "write_question" | "render_reel" | "voice_reel" | "caption_reel"
        | "translate_reel" => pipeline::run_step(ctx, tenant()?, &job.payload).await,
        "provider_callback" => pipeline::provider_callback(ctx, &job.payload).await,
        "build_daily_plans" => plans::build_daily_plans(ctx, &job.payload).await,
        "send_delivery" => deliveries::send_delivery(ctx, tenant()?, &job.payload).await,
        "hrdf_export" => exports::hrdf_export(ctx, tenant()?, &job.payload).await,
        "data_export" => exports::data_export(ctx, tenant()?, &job.payload).await,
        "maintain_partitions" => crate::scheduler::maintain_partitions(ctx).await,
        "engine_hourly" => engine::engine_hourly(ctx, tenant()?).await,
        other => Err(JobError::Permanent(format!(
            "no handler for job kind {other}"
        ))),
    }
}
