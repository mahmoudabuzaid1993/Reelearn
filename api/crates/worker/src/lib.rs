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
    pub providers: Providers,
}

impl Ctx {
    pub fn new(state: AppState, providers: Providers) -> Ctx {
        Ctx { state, providers }
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
