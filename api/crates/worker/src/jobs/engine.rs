//! Hourly learning-engine upkeep per tenant (`{ tenant_id, hour }`):
//!
//! 1. Tag untagged reels with their topic, then recompute the completion baselines per
//!    duration band.
//! 2. Rebuild the 7-day colleague activity table (location, job title, team).
//! 3. Schedule re-engagement reminders, within the company's limits: only inside delivery
//!    windows, only for people with reminders on, away longer than `reengage_after_hours`,
//!    whose minute is not done, and below `max_nudges_per_day`. Sending is `send_delivery`'s job.
//! 4. Ask TypeSafe Jev how relevant new published reels are to each active job title, a
//!    starting point for people the engine knows little about (company setting `ai.role_matching`).

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
    let roles = if settings.ai.role_matching {
        match_roles(ctx, tenant_id).await?
    } else {
        String::new()
    };
    Ok(JobOutcome::done(format!(
        "{tagged} reels tagged, {bands} band baselines, {cohorts} cohort rows, {reminders} reminders scheduled{roles}"
    )))
}

/// Job titles judged per run, and reels per title, so one run stays small.
const ROLES_PER_RUN: i64 = 25;
const REELS_PER_ROLE: i64 = 120;

/// Fill `reel_role_fit` for active job titles and published reels not judged yet. Advisory: a
/// judge that is off or failing leaves the engine on its other signals.
async fn match_roles(ctx: &Ctx, tenant_id: Uuid) -> Result<String, JobError> {
    use crate::providers::RoleReel;
    use reelearn_db::repos::ai as ai_repo;
    if ctx.providers.judge.name() == "off" {
        return Ok(String::new());
    }
    let mut tx = ctx.state.db.tenant_tx(tenant_id).await?;
    let roles = ai_repo::active_roles(tx.conn(), ROLES_PER_RUN).await?;
    let mut scored = 0usize;
    for role in roles {
        let gaps = ai_repo::role_gaps(tx.conn(), &role, REELS_PER_ROLE).await?;
        if gaps.is_empty() {
            continue;
        }
        let reels: Vec<RoleReel> = gaps
            .iter()
            .map(|g| RoleReel {
                title: g.title.clone(),
                topic: g.topic.clone(),
                tags: g.tags.clone(),
                excerpt: g.excerpt.clone().unwrap_or_default(),
            })
            .collect();
        match ctx.providers.judge.role_fit(&role, &reels).await {
            Ok(fits) => {
                for (g, fit) in gaps.iter().zip(fits) {
                    ai_repo::upsert_role_fit(
                        tx.conn(),
                        tenant_id,
                        g.reel_id,
                        &role,
                        fit,
                        Some(ctx.providers.judge.name()),
                    )
                    .await?;
                    scored += 1;
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, "role matching unavailable");
                break;
            }
        }
    }
    tx.commit().await?;
    Ok(format!(", {scored} role matches"))
}
