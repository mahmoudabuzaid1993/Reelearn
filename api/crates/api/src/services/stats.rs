//! Incremental aggregate maintenance, in the same transaction as the event.

use crate::error::ApiError;
use chrono::NaiveDate;
use reelearn_db::repos::{learning, reels, stats, topics, users::UserRow};
use reelearn_db::settings::TenantSettings;
use reelearn_db::TenantTx;
use reelearn_domain::streak;
use uuid::Uuid;

pub struct ActivityDelta {
    pub reels_completed: i32,
    pub answers_total: i32,
    pub answers_correct: i32,
    pub minutes_on_mandatory: f64,
}

/// Apply learning activity to `user_stats` (streak, counters) and return the new row.
pub async fn record_activity(
    tx: &mut TenantTx,
    settings: &TenantSettings,
    user: &UserRow,
    today: NaiveDate,
    delta: ActivityDelta,
) -> Result<stats::UserStatsRow, ApiError> {
    let tenant_id = tx.tenant_id();
    let current = stats::user_stats_for_update(tx.conn(), tenant_id, user.id).await?;
    let weekend = streak::parse_weekend_days(&settings.weekend_days);
    let outcome = streak::on_activity(
        &streak::StreakState {
            streak_current: current.streak_current,
            streak_longest: current.streak_longest,
            last_active_on: current.last_active_on,
            freezes_available: current.freezes_available,
        },
        today,
        &weekend,
        i16::try_from(settings.streak_freezes_per_week).unwrap_or(1),
    );
    let remembered =
        i32::try_from(learning::remembered_count(tx.conn(), user.id).await?).unwrap_or(i32::MAX);
    let row = stats::write_user_stats(
        tx.conn(),
        user.id,
        stats::UserStatsWrite {
            streak_current: outcome.state.streak_current,
            streak_longest: outcome.state.streak_longest,
            last_active_on: outcome.state.last_active_on,
            freezes_available: outcome.state.freezes_available,
            reels_completed_delta: delta.reels_completed,
            answers_total_delta: delta.answers_total,
            answers_correct_delta: delta.answers_correct,
            remembered_count: Some(remembered),
            minutes_on_mandatory_delta: delta.minutes_on_mandatory,
        },
    )
    .await?;
    Ok(row)
}

/// Recompute the (user, topic) readiness row from memory states and watch history.
pub async fn refresh_readiness(
    tx: &mut TenantTx,
    user_id: Uuid,
    topic_id: Uuid,
    today: NaiveDate,
) -> Result<(), ApiError> {
    let tenant_id = tx.tenant_id();
    let by_topic = learning::memory_by_topic(tx.conn(), user_id, today).await?;
    let (total, remembered) = by_topic
        .iter()
        .find(|t| t.topic_id == topic_id)
        .map(|t| (t.total, t.remembered))
        .unwrap_or((0, 0));
    let recall_pct = if total > 0 {
        i16::try_from(100 * remembered / total).unwrap_or(100)
    } else {
        0
    };
    let reels_total = i32::try_from(reels::count_in_topic_published(tx.conn(), topic_id).await?)
        .unwrap_or(i32::MAX);
    let reels_seen =
        i32::try_from(learning::reels_seen_in_topic(tx.conn(), user_id, topic_id).await?)
            .unwrap_or(i32::MAX);
    let ready = reels_total > 0 && reels_seen >= reels_total && recall_pct >= 70;
    stats::upsert_readiness(
        tx.conn(),
        tenant_id,
        user_id,
        topic_id,
        ready,
        recall_pct,
        reels_seen,
        reels_total,
    )
    .await?;
    Ok(())
}

/// Bump the per-(topic, group, day) counters when the user belongs to an assigned group.
pub async fn bump_daily_topic(
    tx: &mut TenantTx,
    user_id: Uuid,
    topic_id: Uuid,
    day: NaiveDate,
    first_activity_today: bool,
    reels_completed: i32,
    answers_total: i32,
    answers_correct: i32,
    minutes: f64,
) -> Result<(), ApiError> {
    let tenant_id = tx.tenant_id();
    let Some(group_id) = topics::group_for_user_topic(tx.conn(), user_id, topic_id).await? else {
        return Ok(());
    };
    stats::bump_daily_topic_stats(
        tx.conn(),
        tenant_id,
        topic_id,
        group_id,
        day,
        stats::DailyTopicDelta {
            learners_active: i32::from(first_activity_today),
            reels_completed,
            answers_total,
            answers_correct,
            minutes_total: minutes,
        },
    )
    .await?;
    Ok(())
}
