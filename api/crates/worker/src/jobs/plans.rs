//! `build_daily_plans`: freeze every learner's minute for the day and schedule the
//! WhatsApp deliveries inside the learner's shift window.

use crate::jobs::{JobError, JobOutcome};
use crate::Ctx;
use chrono::{DateTime, NaiveDate, NaiveTime, TimeZone, Utc};
use reelearn_db::repos::{deliveries, tenants, users};
use reelearn_db::{plans, TenantSettings};
use reelearn_domain::Channel;
use uuid::Uuid;

#[derive(Debug, serde::Deserialize)]
struct Payload {
    tenant_id: Uuid,
    day: NaiveDate,
}

pub async fn build_daily_plans(
    ctx: &Ctx,
    payload: &serde_json::Value,
) -> Result<JobOutcome, JobError> {
    let p: Payload = serde_json::from_value(payload.clone())
        .map_err(|e| JobError::Permanent(format!("bad payload: {e}")))?;
    let (tenant, recipients) = {
        let mut tx = ctx.state.db.tenant_tx(p.tenant_id).await?;
        let tenant = tenants::get(tx.conn(), p.tenant_id).await?;
        let recipients = users::plan_recipients(tx.conn()).await?;
        tx.commit().await?;
        (tenant, recipients)
    };
    let settings = TenantSettings::from_json(&tenant.settings);
    let tz: chrono_tz::Tz = tenant.timezone.parse().unwrap_or(chrono_tz::Asia::Riyadh);

    let mut built = 0;
    let mut scheduled = 0;
    for user in &recipients {
        // One transaction per learner keeps lock scope small and makes partial progress durable.
        let mut tx = ctx.state.db.tenant_tx(p.tenant_id).await?;
        let plan = plans::ensure_plan(tx.conn(), p.tenant_id, &settings, user, p.day).await?;
        if plan.created {
            built += 1;
        }
        if plan.plan.item_count > 0
            && user.delivery_channel.includes_whatsapp()
            && !deliveries::exists_for_plan(tx.conn(), plan.plan.id, "daily_minute").await?
        {
            let at = delivery_time(&tz, p.day, user.shift_start, settings.delivery_hour);
            deliveries::schedule(
                tx.conn(),
                p.tenant_id,
                user.id,
                Some(plan.plan.id),
                Channel::Whatsapp,
                "daily_minute",
                at,
            )
            .await?;
            scheduled += 1;
        }
        tx.commit().await?;
        ctx.state
            .cache
            .del_prefix(&reelearn_api::cache::keys::plan_prefix(
                p.tenant_id,
                user.id,
            ))
            .await;
    }
    Ok(JobOutcome::done(format!(
        "{} learners · {built} plans built · {scheduled} WhatsApp deliveries scheduled",
        recipients.len()
    )))
}

/// The learner's shift start (or the tenant's default delivery hour) on `day`, in UTC.
/// A time already in the past is kept as-is so the fan-out sends it on the next tick.
pub fn delivery_time(
    tz: &chrono_tz::Tz,
    day: NaiveDate,
    shift_start: Option<NaiveTime>,
    default_hour: u32,
) -> DateTime<Utc> {
    let local_time = shift_start
        .unwrap_or_else(|| NaiveTime::from_hms_opt(default_hour.min(23), 0, 0).unwrap_or_default());
    let naive = day.and_time(local_time);
    match tz.from_local_datetime(&naive).single() {
        Some(dt) => dt.with_timezone(&Utc),
        None => Utc.from_utc_datetime(&naive),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn riyadh_eight_am_is_five_utc() {
        let tz = chrono_tz::Asia::Riyadh;
        let day = NaiveDate::from_ymd_opt(2026, 11, 4).expect("date");
        let at = delivery_time(&tz, day, None, 8);
        assert_eq!(at.format("%H:%M").to_string(), "05:00");
        let shift = delivery_time(&tz, day, NaiveTime::from_hms_opt(6, 30, 0), 8);
        assert_eq!(shift.format("%H:%M").to_string(), "03:30");
    }
}
