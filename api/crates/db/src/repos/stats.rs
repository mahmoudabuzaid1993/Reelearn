//! Aggregates: `user_stats`, `user_topic_readiness`, `daily_topic_stats`.
//! Manager and report endpoints read only from here: these rows carry no question ids,
//! chosen answers or timestamps finer than a day.

use crate::error::DbError;
use chrono::{DateTime, NaiveDate, Utc};
use sqlx::PgConnection;
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct UserStatsRow {
    pub user_id: Uuid,
    pub streak_current: i32,
    pub streak_longest: i32,
    pub last_active_on: Option<NaiveDate>,
    pub freezes_available: i16,
    pub reels_completed: i32,
    pub answers_total: i32,
    pub answers_correct: i32,
    pub remembered_count: i32,
    pub minutes_on_mandatory: f64,
    pub updated_at: DateTime<Utc>,
}

const SCOLS: &str = "user_id, streak_current, streak_longest, last_active_on, freezes_available, reels_completed, answers_total, answers_correct, remembered_count, minutes_on_mandatory::float8 AS minutes_on_mandatory, updated_at";

pub async fn user_stats(
    conn: &mut PgConnection,
    user_id: Uuid,
) -> Result<Option<UserStatsRow>, DbError> {
    Ok(sqlx::query_as::<_, UserStatsRow>(&format!(
        "SELECT {SCOLS} FROM user_stats WHERE user_id = $1"
    ))
    .bind(user_id)
    .fetch_optional(conn)
    .await?)
}

pub async fn user_stats_for_update(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    user_id: Uuid,
) -> Result<UserStatsRow, DbError> {
    sqlx::query("INSERT INTO user_stats (user_id, tenant_id) VALUES ($1, $2) ON CONFLICT (user_id) DO NOTHING")
        .bind(user_id)
        .bind(tenant_id)
        .execute(&mut *conn)
        .await?;
    Ok(sqlx::query_as::<_, UserStatsRow>(&format!(
        "SELECT {SCOLS} FROM user_stats WHERE user_id = $1 FOR UPDATE"
    ))
    .bind(user_id)
    .fetch_one(conn)
    .await?)
}

pub struct UserStatsWrite {
    pub streak_current: i32,
    pub streak_longest: i32,
    pub last_active_on: Option<NaiveDate>,
    pub freezes_available: i16,
    pub reels_completed_delta: i32,
    pub answers_total_delta: i32,
    pub answers_correct_delta: i32,
    pub remembered_count: Option<i32>,
    pub minutes_on_mandatory_delta: f64,
}

pub async fn write_user_stats(
    conn: &mut PgConnection,
    user_id: Uuid,
    w: UserStatsWrite,
) -> Result<UserStatsRow, DbError> {
    Ok(sqlx::query_as::<_, UserStatsRow>(&format!(
        "UPDATE user_stats SET streak_current = $2, streak_longest = $3, last_active_on = $4, freezes_available = $5,
           reels_completed = reels_completed + $6, answers_total = answers_total + $7, answers_correct = answers_correct + $8,
           remembered_count = coalesce($9, remembered_count), minutes_on_mandatory = minutes_on_mandatory + ($10::float8)::numeric, updated_at = now()
         WHERE user_id = $1 RETURNING {SCOLS}"
    ))
    .bind(user_id)
    .bind(w.streak_current)
    .bind(w.streak_longest)
    .bind(w.last_active_on)
    .bind(w.freezes_available)
    .bind(w.reels_completed_delta)
    .bind(w.answers_total_delta)
    .bind(w.answers_correct_delta)
    .bind(w.remembered_count)
    .bind(w.minutes_on_mandatory_delta)
    .fetch_one(conn)
    .await?)
}

pub async fn total_minutes_on_mandatory(conn: &mut PgConnection) -> Result<f64, DbError> {
    Ok(
        sqlx::query_scalar("SELECT coalesce(sum(minutes_on_mandatory), 0)::float8 FROM user_stats")
            .fetch_one(conn)
            .await?,
    )
}

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct ReadinessRow {
    pub user_id: Uuid,
    pub topic_id: Uuid,
    pub ready: bool,
    pub recall_pct: i16,
    pub reels_seen: i32,
    pub reels_total: i32,
}

pub async fn upsert_readiness(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    user_id: Uuid,
    topic_id: Uuid,
    ready: bool,
    recall_pct: i16,
    reels_seen: i32,
    reels_total: i32,
) -> Result<(), DbError> {
    sqlx::query(
        "INSERT INTO user_topic_readiness (user_id, topic_id, tenant_id, ready, recall_pct, reels_seen, reels_total)
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         ON CONFLICT (user_id, topic_id) DO UPDATE SET ready = EXCLUDED.ready, recall_pct = EXCLUDED.recall_pct, reels_seen = EXCLUDED.reels_seen, reels_total = EXCLUDED.reels_total, updated_at = now()",
    )
    .bind(user_id)
    .bind(topic_id)
    .bind(tenant_id)
    .bind(ready)
    .bind(recall_pct)
    .bind(reels_seen)
    .bind(reels_total)
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn readiness_for_users(
    conn: &mut PgConnection,
    user_ids: &[Uuid],
) -> Result<Vec<ReadinessRow>, DbError> {
    Ok(sqlx::query_as::<_, ReadinessRow>("SELECT user_id, topic_id, ready, recall_pct, reels_seen, reels_total FROM user_topic_readiness WHERE user_id = ANY($1)")
        .bind(user_ids)
        .fetch_all(conn)
        .await?)
}

pub async fn readiness_for_user(
    conn: &mut PgConnection,
    user_id: Uuid,
) -> Result<Vec<ReadinessRow>, DbError> {
    Ok(sqlx::query_as::<_, ReadinessRow>("SELECT user_id, topic_id, ready, recall_pct, reels_seen, reels_total FROM user_topic_readiness WHERE user_id = $1")
        .bind(user_id)
        .fetch_all(conn)
        .await?)
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct LastActiveRow {
    pub user_id: Uuid,
    pub last_active_on: Option<NaiveDate>,
}

pub async fn last_active_for_users(
    conn: &mut PgConnection,
    user_ids: &[Uuid],
) -> Result<Vec<LastActiveRow>, DbError> {
    Ok(sqlx::query_as::<_, LastActiveRow>(
        "SELECT user_id, last_active_on FROM user_stats WHERE user_id = ANY($1)",
    )
    .bind(user_ids)
    .fetch_all(conn)
    .await?)
}

pub struct DailyTopicDelta {
    pub learners_active: i32,
    pub reels_completed: i32,
    pub answers_total: i32,
    pub answers_correct: i32,
    pub minutes_total: f64,
}

pub async fn bump_daily_topic_stats(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    topic_id: Uuid,
    group_id: Uuid,
    day: NaiveDate,
    d: DailyTopicDelta,
) -> Result<(), DbError> {
    sqlx::query(
        "INSERT INTO daily_topic_stats (tenant_id, topic_id, group_id, day, learners_active, reels_completed, answers_total, answers_correct, minutes_total)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, ($9::float8)::numeric)
         ON CONFLICT (tenant_id, topic_id, group_id, day) DO UPDATE SET
           learners_active = daily_topic_stats.learners_active + EXCLUDED.learners_active,
           reels_completed = daily_topic_stats.reels_completed + EXCLUDED.reels_completed,
           answers_total = daily_topic_stats.answers_total + EXCLUDED.answers_total,
           answers_correct = daily_topic_stats.answers_correct + EXCLUDED.answers_correct,
           minutes_total = daily_topic_stats.minutes_total + EXCLUDED.minutes_total",
    )
    .bind(tenant_id)
    .bind(topic_id)
    .bind(group_id)
    .bind(day)
    .bind(d.learners_active)
    .bind(d.reels_completed)
    .bind(d.answers_total)
    .bind(d.answers_correct)
    .bind(d.minutes_total)
    .execute(conn)
    .await?;
    Ok(())
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ReportRowRaw {
    pub id: Uuid,
    pub name: String,
    pub learners_active: i64,
    pub learners_total: i64,
    pub recall_pct: i32,
    pub reels_completed: i64,
    pub minutes_total: f64,
    pub ready_pct: i32,
}

pub async fn report_by_topic(
    conn: &mut PgConnection,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<Vec<ReportRowRaw>, DbError> {
    Ok(sqlx::query_as::<_, ReportRowRaw>(
        "SELECT t.id, t.name,
           (SELECT count(*) FROM user_topic_readiness r WHERE r.topic_id = t.id AND r.updated_at::date BETWEEN $1 AND $2) AS learners_active,
           (SELECT count(*) FROM user_topic_readiness r WHERE r.topic_id = t.id) AS learners_total,
           coalesce((SELECT CASE WHEN sum(s.answers_total) > 0 THEN (100 * sum(s.answers_correct) / sum(s.answers_total))::int ELSE 0 END FROM daily_topic_stats s WHERE s.topic_id = t.id AND s.day BETWEEN $1 AND $2), 0) AS recall_pct,
           coalesce((SELECT sum(s.reels_completed) FROM daily_topic_stats s WHERE s.topic_id = t.id AND s.day BETWEEN $1 AND $2), 0)::bigint AS reels_completed,
           coalesce((SELECT sum(s.minutes_total) FROM daily_topic_stats s WHERE s.topic_id = t.id AND s.day BETWEEN $1 AND $2), 0)::float8 AS minutes_total,
           coalesce((SELECT CASE WHEN count(*) > 0 THEN (100 * count(*) FILTER (WHERE r.ready) / count(*))::int ELSE 0 END FROM user_topic_readiness r WHERE r.topic_id = t.id), 0) AS ready_pct
         FROM topics t WHERE t.status = 'active' ORDER BY t.name",
    )
    .bind(from)
    .bind(to)
    .fetch_all(conn)
    .await?)
}

pub async fn report_by_group(
    conn: &mut PgConnection,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<Vec<ReportRowRaw>, DbError> {
    Ok(sqlx::query_as::<_, ReportRowRaw>(
        "SELECT g.id, g.name,
           (SELECT count(*) FROM group_members gm JOIN user_stats us ON us.user_id = gm.user_id WHERE gm.group_id = g.id AND us.last_active_on BETWEEN $1 AND $2) AS learners_active,
           (SELECT count(*) FROM group_members gm WHERE gm.group_id = g.id) AS learners_total,
           coalesce((SELECT CASE WHEN sum(s.answers_total) > 0 THEN (100 * sum(s.answers_correct) / sum(s.answers_total))::int ELSE 0 END FROM daily_topic_stats s WHERE s.group_id = g.id AND s.day BETWEEN $1 AND $2), 0) AS recall_pct,
           coalesce((SELECT sum(s.reels_completed) FROM daily_topic_stats s WHERE s.group_id = g.id AND s.day BETWEEN $1 AND $2), 0)::bigint AS reels_completed,
           coalesce((SELECT sum(s.minutes_total) FROM daily_topic_stats s WHERE s.group_id = g.id AND s.day BETWEEN $1 AND $2), 0)::float8 AS minutes_total,
           coalesce((SELECT CASE WHEN count(*) > 0 THEN (100 * count(*) FILTER (WHERE r.ready) / count(*))::int ELSE 0 END
                     FROM user_topic_readiness r JOIN group_members gm ON gm.user_id = r.user_id WHERE gm.group_id = g.id), 0) AS ready_pct
         FROM groups g ORDER BY g.name",
    )
    .bind(from)
    .bind(to)
    .fetch_all(conn)
    .await?)
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct OverviewRow {
    pub reels_completed: i64,
    pub answers_total: i64,
    pub answers_correct: i64,
    pub minutes_total: f64,
}

pub async fn overview(
    conn: &mut PgConnection,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<OverviewRow, DbError> {
    Ok(sqlx::query_as::<_, OverviewRow>(
        "SELECT coalesce(sum(reels_completed), 0)::bigint AS reels_completed, coalesce(sum(answers_total), 0)::bigint AS answers_total,
                coalesce(sum(answers_correct), 0)::bigint AS answers_correct, coalesce(sum(minutes_total), 0)::float8 AS minutes_total
         FROM daily_topic_stats WHERE day BETWEEN $1 AND $2",
    )
    .bind(from)
    .bind(to)
    .fetch_one(conn)
    .await?)
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct DaySeriesRow {
    pub day: NaiveDate,
    pub active: i64,
    pub answers_total: i64,
    pub answers_correct: i64,
}

pub async fn day_series(
    conn: &mut PgConnection,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<Vec<DaySeriesRow>, DbError> {
    Ok(sqlx::query_as::<_, DaySeriesRow>(
        "SELECT day, coalesce(sum(learners_active), 0)::bigint AS active, coalesce(sum(answers_total), 0)::bigint AS answers_total, coalesce(sum(answers_correct), 0)::bigint AS answers_correct
         FROM daily_topic_stats WHERE day BETWEEN $1 AND $2 GROUP BY day ORDER BY day",
    )
    .bind(from)
    .bind(to)
    .fetch_all(conn)
    .await?)
}

/// Learners inactive for at least `days` days (attention list), names only.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct InactiveRow {
    pub user_id: Uuid,
    pub full_name: String,
    pub last_active_on: Option<NaiveDate>,
}

pub async fn inactive_learners(
    conn: &mut PgConnection,
    before: NaiveDate,
    limit: i64,
) -> Result<Vec<InactiveRow>, DbError> {
    Ok(sqlx::query_as::<_, InactiveRow>(
        "SELECT u.id AS user_id, u.full_name, s.last_active_on FROM users u LEFT JOIN user_stats s ON s.user_id = u.id
         WHERE u.role = 'learner' AND u.status = 'active' AND (s.last_active_on IS NULL OR s.last_active_on < $1) ORDER BY s.last_active_on NULLS FIRST LIMIT $2",
    )
    .bind(before)
    .bind(limit)
    .fetch_all(conn)
    .await?)
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct HrdfRow {
    pub day: NaiveDate,
    pub topic: String,
    pub compliance_tag: Option<String>,
    pub group_name: String,
    pub learners_active: i32,
    pub reels_completed: i32,
    pub answers_total: i32,
    pub answers_correct: i32,
    pub minutes_total: f64,
}

/// Rows for the HRDF evidence pack: mandatory topics per group per day.
pub async fn hrdf_rows(
    conn: &mut PgConnection,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<Vec<HrdfRow>, DbError> {
    Ok(sqlx::query_as::<_, HrdfRow>(
        "SELECT s.day, t.name AS topic, t.compliance_tag, g.name AS group_name, s.learners_active, s.reels_completed, s.answers_total, s.answers_correct, s.minutes_total::float8 AS minutes_total
         FROM daily_topic_stats s JOIN topics t ON t.id = s.topic_id JOIN groups g ON g.id = s.group_id
         WHERE s.day BETWEEN $1 AND $2 AND t.is_mandatory ORDER BY s.day, t.name, g.name",
    )
    .bind(from)
    .bind(to)
    .fetch_all(conn)
    .await?)
}
