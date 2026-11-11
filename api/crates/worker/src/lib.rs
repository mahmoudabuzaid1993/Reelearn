//! Reelearn background worker.
//!
//! - [`queue`]: the durable job loop over the `jobs` table (`FOR UPDATE SKIP LOCKED`, backoff, dead-letter).
//! - [`scheduler`]: the leader that enqueues daily plan builds and delivery fan-out.
//! - [`jobs`]: one handler per job kind; [`jobs::pipeline`] is the AI Reel Studio.
//! - [`providers`]: the vendor traits with deterministic fakes and HTTP adapters.
//!
//! The worker shares [`reelearn_api::AppState`] with the API (same config, pool, cache
//! and object storage), so a job sees exactly what a request handler sees.

pub mod jobs;
pub mod providers;
pub mod queue;
pub mod scheduler;

use providers::Providers;
use reelearn_api::AppState;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch;

/// Everything a job needs.
pub struct Ctx {
    pub state: AppState,
    /// The providers every company gets in fixed mode (development fakes, tests).
    pub providers: Providers,
    /// Resolve each company's keys (its own, else the platform's) per job.
    pub per_company: bool,
}

impl Ctx {
    /// Fixed providers for every company: development and tests.
    pub fn new(state: AppState, providers: Providers) -> Ctx {
        Ctx {
            state,
            providers,
            per_company: false,
        }
    }

    /// Production: each job runs with the keys its company resolves to (see `services::ai`).
    pub fn per_company(state: AppState) -> Ctx {
        Ctx {
            state,
            providers: Providers::fake(),
            per_company: true,
        }
    }

    /// The providers for one company's job.
    pub async fn providers_for(
        &self,
        tenant_id: uuid::Uuid,
    ) -> Result<Providers, reelearn_api::error::ApiError> {
        if !self.per_company {
            return Ok(self.providers.clone());
        }
        let resolved = reelearn_api::services::ai::resolve(&self.state, tenant_id).await?;
        Ok(Providers::from_resolved(self.state.http.clone(), &resolved))
    }

    /// A copy of this context bound to one company's providers.
    pub async fn scoped(
        &self,
        tenant_id: uuid::Uuid,
    ) -> Result<Ctx, reelearn_api::error::ApiError> {
        Ok(Ctx {
            state: self.state.clone(),
            providers: self.providers_for(tenant_id).await?,
            per_company: false,
        })
    }

    /// Write what the providers recorded to `ai_usage`, in its own transaction so a failed
    /// step still accounts for what the vendor charged. Never fails the job.
    pub async fn flush_usage(
        &self,
        tenant_id: uuid::Uuid,
        p: &Providers,
        job_id: Option<uuid::Uuid>,
    ) {
        for e in p.meter.drain() {
            let Some(source) = p.sources.get(e.provider).copied() else {
                continue;
            };
            let prices = p.prices.get(e.provider).copied().unwrap_or_default();
            if let Err(err) = reelearn_api::services::ai::record_usage(
                &self.state,
                tenant_id,
                e.provider,
                e.capability,
                source,
                prices,
                e.model.as_deref(),
                e.input_tokens,
                e.output_tokens,
                e.units,
                e.ok,
                job_id,
            )
            .await
            {
                tracing::warn!(error = %err, provider = e.provider, "could not record AI usage");
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct RunOptions {
    pub worker_id: String,
    pub concurrency: usize,
    pub poll_interval: Duration,
    pub scheduler_tick: Duration,
    pub run_scheduler: bool,
}

impl Default for RunOptions {
    fn default() -> Self {
        RunOptions {
            worker_id: format!("worker-{}", uuid::Uuid::new_v4().simple()),
            concurrency: 4,
            poll_interval: Duration::from_secs(1),
            scheduler_tick: Duration::from_secs(30),
            run_scheduler: true,
        }
    }
}

/// Run `concurrency` worker loops plus (optionally) the scheduler until `stop` flips.
pub async fn run(ctx: Arc<Ctx>, opts: RunOptions, stop: watch::Receiver<bool>) {
    let mut tasks = Vec::new();
    for i in 0..opts.concurrency.max(1) {
        let id = format!("{}-{i}", opts.worker_id);
        tasks.push(tokio::spawn(queue::run_worker(
            ctx.clone(),
            id,
            stop.clone(),
            opts.poll_interval,
        )));
    }
    if opts.run_scheduler {
        tasks.push(tokio::spawn(scheduler::run_scheduler(
            ctx.clone(),
            opts.worker_id.clone(),
            stop.clone(),
            opts.scheduler_tick,
        )));
    }
    for t in tasks {
        let _ = t.await;
    }
}

/// Drain the queue: run jobs until none is due. Used by tests and by `reelearn-worker drain`.
pub async fn drain(
    ctx: &Ctx,
    worker_id: &str,
    max_jobs: usize,
) -> Result<usize, reelearn_db::DbError> {
    let mut n = 0;
    while n < max_jobs && queue::run_one(ctx, worker_id).await? {
        n += 1;
    }
    Ok(n)
}
