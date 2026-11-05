//! The worker loop: claim one job with `SKIP LOCKED`, run it, complete or retry with backoff.

use crate::jobs::{dispatch, pipeline, JobError};
use crate::Ctx;
use chrono::{Duration as ChronoDuration, Utc};
use rand::Rng;
use reelearn_db::repos::jobs;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch;

/// Exponential backoff with jitter: 5 s, 10 s, 20 s … capped at 30 minutes.
pub fn backoff(attempts: i16) -> Duration {
    let exp = u32::try_from(attempts.max(1) - 1).unwrap_or(0).min(12);
    let base = 5u64.saturating_mul(1u64 << exp).min(30 * 60);
    let jitter = rand::thread_rng().gen_range(0..=base / 4 + 1);
    Duration::from_secs(base + jitter)
}

/// Run one job if one is due. Returns whether a job was found.
pub async fn run_one(ctx: &Ctx, worker_id: &str) -> Result<bool, reelearn_db::DbError> {
    let mut admin = ctx.state.db.admin_tx().await?;
    let Some(job) = jobs::claim(&mut admin, worker_id, None).await? else {
        admin.commit().await?;
        return Ok(false);
    };
    admin.commit().await?;

    let started = std::time::Instant::now();
    let result = dispatch(ctx, &job).await;
    pipeline::flush_uploads(ctx).await;
    let mut admin = ctx.state.db.admin_tx().await?;
    match result {
        Ok(outcome) => {
            jobs::complete(&mut admin, job.id).await?;
            tracing::info!(job = %job.id, kind = %job.kind, ms = started.elapsed().as_millis() as u64, note = %outcome.note, "job done");
        }
        Err(JobError::Transient(msg)) => {
            let retry_at = Utc::now()
                + ChronoDuration::from_std(backoff(job.attempts))
                    .unwrap_or(ChronoDuration::seconds(30));
            let status = jobs::fail(&mut admin, job.id, &msg, Some(retry_at)).await?;
            tracing::warn!(job = %job.id, kind = %job.kind, attempts = job.attempts, status = status.as_str(), error = %msg, "job will retry");
        }
        Err(JobError::Permanent(msg)) => {
            jobs::fail(&mut admin, job.id, &msg, None).await?;
            tracing::error!(job = %job.id, kind = %job.kind, error = %msg, "job failed permanently");
        }
    }
    admin.commit().await?;
    Ok(true)
}

/// The loop a worker task runs until `stop` flips to true.
pub async fn run_worker(
    ctx: Arc<Ctx>,
    worker_id: String,
    mut stop: watch::Receiver<bool>,
    poll: Duration,
) {
    tracing::info!(%worker_id, "worker loop started");
    loop {
        if *stop.borrow() {
            break;
        }
        match run_one(&ctx, &worker_id).await {
            Ok(true) => continue,
            Ok(false) => {}
            Err(e) => tracing::error!(error = %e, "queue error; backing off"),
        }
        tokio::select! {
            _ = tokio::time::sleep(poll) => {}
            _ = stop.changed() => {}
        }
    }
    tracing::info!(%worker_id, "worker loop stopped");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_grows_and_caps() {
        assert!(backoff(1) >= Duration::from_secs(5) && backoff(1) <= Duration::from_secs(7));
        assert!(backoff(4) >= Duration::from_secs(40));
        assert!(backoff(30) <= Duration::from_secs(30 * 60 + 30 * 60 / 4 + 1));
    }
}
