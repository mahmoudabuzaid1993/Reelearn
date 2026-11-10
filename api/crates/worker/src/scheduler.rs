//! The scheduler leader: one worker holds a Redis lease and, every tick, enqueues the
//! per-tenant `build_daily_plans` job at the tenant's local build hour, fans out due
//! deliveries, runs the hourly learning-engine upkeep, releases stale jobs and keeps next
//! month's partitions ready.

use crate::jobs::{JobError, JobOutcome};
use crate::Ctx;
use chrono::{Datelike, Timelike, Utc};
use reelearn_db::repos::{deliveries, jobs, tenants};
use reelearn_db::{settings, TenantSettings};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch;

const LEASE_KEY: &str = "lease:scheduler";
const LEASE_TTL: Duration = Duration::from_secs(60);

pub async fn run_scheduler(
    ctx: Arc<Ctx>,
    worker_id: String,
    mut stop: watch::Receiver<bool>,
    tick: Duration,
) {
    loop {
        if *stop.borrow() {
            break;
        }
        if acquire_lease(&ctx, &worker_id).await {
            if let Err(e) = tick_once(&ctx).await {
                tracing::error!(error = %e, "scheduler tick failed");
            }
        }
        tokio::select! {
            _ = tokio::time::sleep(tick) => {}
            _ = stop.changed() => {}
        }
    }
}

/// Take or renew the lease. Without Redis every worker schedules; the enqueue checks keep that safe.
async fn acquire_lease(ctx: &Ctx, worker_id: &str) -> bool {
    if !ctx.state.cache.is_connected() {
        return true;
    }
    match ctx
        .state
        .cache
        .set_nx(LEASE_KEY, worker_id, LEASE_TTL)
        .await
    {
        Some(true) => true,
        Some(false) => {
            if ctx.state.cache.get_string(LEASE_KEY).await.as_deref() == Some(worker_id) {
                ctx.state
                    .cache
                    .set_string(LEASE_KEY, worker_id, LEASE_TTL)
                    .await;
                true
            } else {
                false
            }
        }
        None => true,
    }
}

pub async fn tick_once(ctx: &Ctx) -> Result<(), reelearn_db::DbError> {
    let now = Utc::now();
    let mut admin = ctx.state.db.admin_tx().await?;
    let released = jobs::release_stale(&mut admin, 15).await?;
    if released > 0 {
        tracing::warn!(released, "released jobs whose worker died");
    }

    // Daily plans per tenant, at the tenant's local build hour.
    for tenant in tenants::list_all(&mut admin).await? {
        let settings = TenantSettings::from_json(&tenant.settings);
        let local = settings::local_now(&tenant.timezone, now);
        let day = local.date();
        if local.hour() >= settings.plan_build_hour
            && !jobs::exists_for_day(&mut admin, tenant.id, "build_daily_plans", day).await?
        {
            jobs::enqueue(
                &mut admin,
                Some(tenant.id),
                "build_daily_plans",
                serde_json::json!({ "tenant_id": tenant.id, "day": day }),
                None,
            )
            .await?;
            tracing::info!(tenant = %tenant.slug, %day, "enqueued daily plan build");
        }
    }

    // Learning-engine upkeep: once an hour per tenant (baselines, cohorts, reminders).
    let hour = now.format("%Y-%m-%dT%H").to_string();
    for tenant in tenants::list_all(&mut admin).await? {
        if !jobs::exists_with_key(&mut admin, tenant.id, "engine_hourly", "hour", &hour).await? {
            jobs::enqueue(
                &mut admin,
                Some(tenant.id),
                "engine_hourly",
                serde_json::json!({ "tenant_id": tenant.id, "hour": hour }),
                None,
            )
            .await?;
        }
    }

    // Deliveries whose time has come and that no job carries yet.
    for d in deliveries::due_admin(&mut admin, 500).await? {
        if jobs::active_for_delivery(&mut admin, d.id).await? {
            continue;
        }
        jobs::enqueue(&mut admin, Some(d.tenant_id), "send_delivery", serde_json::json!({ "delivery_id": d.id, "kind": d.kind, "scheduled_for": d.scheduled_for }), None).await?;
    }

    // Partitions: once a day.
    let stamp = format!("partitions:{}", now.format("%Y-%m-%d"));
    if ctx
        .state
        .cache
        .set_nx(&stamp, "1", Duration::from_secs(36 * 3600))
        .await
        .unwrap_or(true)
        && !jobs::exists_for_day(
            &mut admin,
            uuid::Uuid::nil(),
            "maintain_partitions",
            now.date_naive(),
        )
        .await?
    {
        jobs::enqueue(
            &mut admin,
            None,
            "maintain_partitions",
            serde_json::json!({ "day": now.date_naive() }),
            None,
        )
        .await?;
    }
    admin.commit().await?;
    Ok(())
}

/// Create this month's and next month's partitions for the event tables.
pub async fn maintain_partitions(ctx: &Ctx) -> Result<JobOutcome, JobError> {
    let mut admin = ctx.state.db.admin_tx().await?;
    let today = Utc::now().date_naive();
    let next = if today.month() == 12 {
        today
            .with_year(today.year() + 1)
            .and_then(|d| d.with_month(1))
    } else {
        today.with_month(today.month() + 1)
    }
    .unwrap_or(today);
    for table in [
        "watch_events",
        "answers",
        "deliveries",
        "audit_log",
        "engagement_events",
    ] {
        for month in [today, next] {
            sqlx::query("SELECT ensure_month_partition($1, $2)")
                .bind(table)
                .bind(month)
                .execute(&mut *admin)
                .await
                .map_err(|e| JobError::Transient(e.to_string()))?;
        }
    }
    admin
        .commit()
        .await
        .map_err(|e| JobError::Transient(e.to_string()))?;
    Ok(JobOutcome::done(
        "partitions ensured for this month and next".into(),
    ))
}
