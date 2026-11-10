//! Hourly learning-engine upkeep per tenant (`{ tenant_id, hour }`):
//!
//! 1. Tag untagged reels with their topic, then recompute the completion baselines per
//!    duration band.
//! 2. Rebuild the 7-day colleague activity table (location, job title, team).
//! 3. Schedule re-engagement reminders, within the company's limits: only inside delivery
//!    windows, only for people with reminders on, away longer than `reengage_after_hours`,
//!    whose minute is not done, and below `max_nudges_per_day`. Sending is `send_delivery`'s job.

use crate::jobs::{JobError, JobOutcome};
use crate::Ctx;
use chrono::Utc;
use reelearn_db::repos::{deliveries, engine as repo, tenants};
use reelearn_db::settings::{local_day_start, local_today};
use reelearn_db::TenantSettings;
use uuid::Uuid;

/// Reminders scheduled per tenant per run, so one hour never floods the queue.
const REMINDERS_PER_RUN: i64 = 500;

pub async fn engine_hourly(ctx: &Ctx, tenant_id: Uuid) -> Result<JobOutcome, JobError> {
    let mut tx = ctx.state.db.tenant_tx(tenant_id).await?;
    let tenant = tenants::get(tx.conn(), tenant_id).await?;
    let settings = TenantSettings::from_json(&tenant.settings);
    let tagged = repo::backfill_topic_tags(tx.conn(), tenant_id).await?;
    let bands = repo::refresh_band_stats(tx.conn(), tenant_id).await?;
    let cohorts = repo::refresh_cohort_stats(tx.conn(), tenant_id).await?;

    let es = &settings.engine;
    let now = Utc::now();
    let mut reminders = 0;
    if es.reengage_after_hours > 0
        && es.max_nudges_per_day > 0
        && settings.within_windows(&tenant.timezone, now)
    {
        let today = local_today(&tenant.timezone, now);
        let due = repo::reengage_candidates(
            tx.conn(),
            today,
            local_day_start(&tenant.timezone, today),
            i32::from(es.reengage_after_hours),
            i64::from(es.max_nudges_per_day),
            REMINDERS_PER_RUN,
        )
        .await?;
        for r in due {
            deliveries::schedule(
                tx.conn(),
                tenant_id,
                r.user_id,
                None,
                r.channel,
                "reminder",
                now,
            )
            .await?;
            reminders += 1;
        }
    }
    tx.commit().await?;
    Ok(JobOutcome::done(format!(
        "{tagged} reels tagged, {bands} band baselines, {cohorts} cohort rows, {reminders} reminders scheduled"
    )))
}
