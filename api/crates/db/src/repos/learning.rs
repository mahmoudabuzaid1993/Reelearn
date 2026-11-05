//! Memory states, watch events, answers, saved reels, reel requests.

use crate::error::DbError;
use chrono::{DateTime, NaiveDate, Utc};
use reelearn_domain::recall::MemoryState;
use reelearn_domain::Channel;
use sqlx::PgConnection;
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct MemoryStateRow {
    pub user_id: Uuid,
    pub question_id: Uuid,
    pub due_on: NaiveDate,
    pub interval_days: i16,
    pub ease: f32,
    pub reps: i16,
    pub lapses: i16,
    pub last_result: Option<bool>,
    pub last_answered_at: Option<DateTime<Utc>>,
}

impl From<&MemoryStateRow> for MemoryState {
    fn from(r: &MemoryStateRow) -> Self {
        MemoryState {
            due_on: r.due_on,
            interval_days: r.interval_days,
            ease: r.ease,
            reps: r.reps,
            lapses: r.lapses,
            last_result: r.last_result,
        }
    }
}

const MCOLS: &str = "user_id, question_id, due_on, interval_days, ease, reps, lapses, last_result, last_answered_at";

pub async fn memory_state(
    conn: &mut PgConnection,
    user_id: Uuid,
    question_id: Uuid,
) -> Result<Option<MemoryStateRow>, DbError> {
    Ok(sqlx::query_as::<_, MemoryStateRow>(&format!(
        "SELECT {MCOLS} FROM memory_states WHERE user_id = $1 AND question_id = $2 FOR UPDATE"
    ))
    .bind(user_id)
    .bind(question_id)
    .fetch_optional(conn)
    .await?)
}

pub async fn upsert_memory_state(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    user_id: Uuid,
    question_id: Uuid,
    s: &MemoryState,
    answered_at: Option<DateTime<Utc>>,
) -> Result<MemoryStateRow, DbError> {
    Ok(sqlx::query_as::<_, MemoryStateRow>(&format!(
        "INSERT INTO memory_states (user_id, question_id, tenant_id, due_on, interval_days, ease, reps, lapses, last_result, last_answered_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
         ON CONFLICT (user_id, question_id) DO UPDATE SET due_on = EXCLUDED.due_on, interval_days = EXCLUDED.interval_days, ease = EXCLUDED.ease,
           reps = EXCLUDED.reps, lapses = EXCLUDED.lapses, last_result = EXCLUDED.last_result, last_answered_at = EXCLUDED.last_answered_at
         RETURNING {MCOLS}"
    ))
    .bind(user_id)
    .bind(question_id)
    .bind(tenant_id)
    .bind(s.due_on)
    .bind(s.interval_days)
    .bind(s.ease)
    .bind(s.reps)
    .bind(s.lapses)
    .bind(s.last_result)
    .bind(answered_at)
    .fetch_one(conn)
    .await?)
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct DueRow {
    pub question_id: Uuid,
    pub reel_id: Uuid,
    pub due_on: NaiveDate,
    pub interval_days: i16,
}

/// Reviews due on or before `day`, for published reels only.
pub async fn due_reviews(
    conn: &mut PgConnection,
    user_id: Uuid,
    day: NaiveDate,
) -> Result<Vec<DueRow>, DbError> {
    Ok(sqlx::query_as::<_, DueRow>(
        "SELECT m.question_id, q.reel_id, m.due_on, m.interval_days FROM memory_states m
         JOIN questions q ON q.id = m.question_id JOIN reels r ON r.id = q.reel_id AND r.status = 'published'
         WHERE m.user_id = $1 AND m.due_on <= $2
           AND NOT EXISTS (SELECT 1 FROM daily_plan_items i JOIN daily_plans p ON p.id = i.plan_id WHERE p.user_id = $1 AND p.plan_date = $2 AND i.question_id = m.question_id)
         ORDER BY m.due_on, m.question_id",
    )
    .bind(user_id)
    .bind(day)
    .fetch_all(conn)
    .await?)
}

pub async fn next_due_on(
    conn: &mut PgConnection,
    user_id: Uuid,
    after: NaiveDate,
) -> Result<Option<NaiveDate>, DbError> {
    Ok(sqlx::query_scalar(
        "SELECT min(due_on) FROM memory_states WHERE user_id = $1 AND due_on > $2",
    )
    .bind(user_id)
    .bind(after)
    .fetch_one(conn)
    .await?)
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TopicMemoryRow {
    pub topic_id: Uuid,
    pub next_due_on: Option<NaiveDate>,
    pub total: i64,
    pub remembered: i64,
}

pub async fn memory_by_topic(
    conn: &mut PgConnection,
    user_id: Uuid,
    after: NaiveDate,
) -> Result<Vec<TopicMemoryRow>, DbError> {
    Ok(sqlx::query_as::<_, TopicMemoryRow>(
        "SELECT r.topic_id, min(m.due_on) FILTER (WHERE m.due_on > $2) AS next_due_on, count(*) AS total,
                count(*) FILTER (WHERE m.reps >= 2 AND m.last_result) AS remembered
         FROM memory_states m JOIN questions q ON q.id = m.question_id JOIN reels r ON r.id = q.reel_id
         WHERE m.user_id = $1 GROUP BY r.topic_id",
    )
    .bind(user_id)
    .bind(after)
    .fetch_all(conn)
    .await?)
}

pub async fn remembered_count(conn: &mut PgConnection, user_id: Uuid) -> Result<i64, DbError> {
    Ok(sqlx::query_scalar(
        "SELECT count(*) FROM memory_states WHERE user_id = $1 AND reps >= 2 AND last_result",
    )
    .bind(user_id)
    .fetch_one(conn)
    .await?)
}

pub async fn memory_count(conn: &mut PgConnection, user_id: Uuid) -> Result<i64, DbError> {
    Ok(
        sqlx::query_scalar("SELECT count(*) FROM memory_states WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(conn)
            .await?,
    )
}

pub async fn insert_watch_event(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    user_id: Uuid,
    reel_id: Uuid,
    plan_item_id: Option<Uuid>,
    channel: Channel,
    device_id: Option<&str>,
    watched_ms: i32,
    completed: bool,
) -> Result<(), DbError> {
    sqlx::query("INSERT INTO watch_events (tenant_id, user_id, reel_id, plan_item_id, channel, device_id, watched_ms, completed) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)")
        .bind(tenant_id)
        .bind(user_id)
        .bind(reel_id)
        .bind(plan_item_id)
        .bind(channel)
        .bind(device_id)
        .bind(watched_ms)
        .bind(completed)
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn has_completed_watch_today(
    conn: &mut PgConnection,
    user_id: Uuid,
    reel_id: Uuid,
) -> Result<bool, DbError> {
    Ok(sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM watch_events WHERE user_id = $1 AND reel_id = $2 AND completed AND started_at > now() - interval '1 day')")
        .bind(user_id)
        .bind(reel_id)
        .fetch_one(conn)
        .await?)
}

/// One row per answer; every column is a value the caller already holds, so a struct would only rename them.
#[allow(clippy::too_many_arguments)]
pub async fn insert_answer(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    user_id: Uuid,
    question_id: Uuid,
    reel_id: Uuid,
    plan_item_id: Option<Uuid>,
    chosen_key: &str,
    is_correct: bool,
    response_ms: Option<i32>,
    attempt_no: i16,
    channel: Channel,
    answered_at: DateTime<Utc>,
) -> Result<(), DbError> {
    sqlx::query(
        "INSERT INTO answers (tenant_id, user_id, question_id, reel_id, plan_item_id, chosen_key, is_correct, response_ms, attempt_no, channel, answered_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
    )
    .bind(tenant_id)
    .bind(user_id)
    .bind(question_id)
    .bind(reel_id)
    .bind(plan_item_id)
    .bind(chosen_key)
    .bind(is_correct)
    .bind(response_ms)
    .bind(attempt_no)
    .bind(channel)
    .bind(answered_at)
    .execute(conn)
    .await?;
    Ok(())
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct WeeklyRecallRow {
    pub week: String,
    pub total: i64,
    pub correct: i64,
}

/// The learner's own weekly recall series (per-answer rows are reachable only through `/me`).
pub async fn weekly_recall(
    conn: &mut PgConnection,
    user_id: Uuid,
    weeks: i32,
) -> Result<Vec<WeeklyRecallRow>, DbError> {
    Ok(sqlx::query_as::<_, WeeklyRecallRow>(
        "SELECT to_char(date_trunc('week', answered_at), 'IYYY-\"W\"IW') AS week, count(*) AS total, count(*) FILTER (WHERE is_correct) AS correct
         FROM answers WHERE user_id = $1 AND answered_at > now() - make_interval(weeks => $2)
         GROUP BY date_trunc('week', answered_at) ORDER BY date_trunc('week', answered_at)",
    )
    .bind(user_id)
    .bind(weeks)
    .fetch_all(conn)
    .await?)
}

/// Correct answers among a plan's items (for the "yesterday" summary on the Today screen).
pub async fn plan_correct_count(conn: &mut PgConnection, plan_id: Uuid) -> Result<i64, DbError> {
    Ok(sqlx::query_scalar(
        "SELECT count(*) FROM answers a WHERE a.plan_item_id IN (SELECT id FROM daily_plan_items WHERE plan_id = $1) AND a.is_correct",
    )
    .bind(plan_id)
    .fetch_one(conn)
    .await?)
}

pub async fn save_reel(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    user_id: Uuid,
    reel_id: Uuid,
) -> Result<(), DbError> {
    sqlx::query("INSERT INTO saved_reels (user_id, reel_id, tenant_id) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING")
        .bind(user_id)
        .bind(reel_id)
        .bind(tenant_id)
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn unsave_reel(
    conn: &mut PgConnection,
    user_id: Uuid,
    reel_id: Uuid,
) -> Result<(), DbError> {
    sqlx::query("DELETE FROM saved_reels WHERE user_id = $1 AND reel_id = $2")
        .bind(user_id)
        .bind(reel_id)
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn is_saved(
    conn: &mut PgConnection,
    user_id: Uuid,
    reel_id: Uuid,
) -> Result<bool, DbError> {
    Ok(sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM saved_reels WHERE user_id = $1 AND reel_id = $2)",
    )
    .bind(user_id)
    .bind(reel_id)
    .fetch_one(conn)
    .await?)
}

pub async fn create_reel_request(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    user_id: Uuid,
    text: &str,
) -> Result<Uuid, DbError> {
    Ok(sqlx::query_scalar(
        "INSERT INTO reel_requests (tenant_id, user_id, text) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(tenant_id)
    .bind(user_id)
    .bind(text)
    .fetch_one(conn)
    .await?)
}

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct ReelRequestRow {
    pub id: Uuid,
    pub text: String,
    pub votes: i32,
    pub status: String,
}

/// Requests from a set of users, anonymised: no user id leaves this function.
pub async fn reel_requests_for_users(
    conn: &mut PgConnection,
    user_ids: &[Uuid],
) -> Result<Vec<ReelRequestRow>, DbError> {
    Ok(sqlx::query_as::<_, ReelRequestRow>("SELECT id, text, votes, status FROM reel_requests WHERE user_id = ANY($1) AND status <> 'dismissed' ORDER BY votes DESC, created_at DESC")
        .bind(user_ids)
        .fetch_all(conn)
        .await?)
}

pub async fn all_reel_requests(conn: &mut PgConnection) -> Result<Vec<ReelRequestRow>, DbError> {
    Ok(sqlx::query_as::<_, ReelRequestRow>("SELECT id, text, votes, status FROM reel_requests WHERE status <> 'dismissed' ORDER BY votes DESC, created_at DESC")
        .fetch_all(conn)
        .await?)
}

/// Distinct reels completed by the user within a topic.
pub async fn reels_seen_in_topic(
    conn: &mut PgConnection,
    user_id: Uuid,
    topic_id: Uuid,
) -> Result<i64, DbError> {
    Ok(sqlx::query_scalar(
        "SELECT count(DISTINCT w.reel_id) FROM watch_events w JOIN reels r ON r.id = w.reel_id WHERE w.user_id = $1 AND r.topic_id = $2 AND w.completed",
    )
    .bind(user_id)
    .bind(topic_id)
    .fetch_one(conn)
    .await?)
}

/// Seed helper: a completed play on a past day.
pub async fn insert_watch_event_at(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    user_id: Uuid,
    reel_id: Uuid,
    watched_ms: i32,
    started_at: DateTime<Utc>,
) -> Result<(), DbError> {
    sqlx::query("INSERT INTO watch_events (tenant_id, user_id, reel_id, channel, watched_ms, completed, started_at) VALUES ($1, $2, $3, 'app', $4, true, $5)")
        .bind(tenant_id)
        .bind(user_id)
        .bind(reel_id)
        .bind(watched_ms)
        .bind(started_at)
        .execute(conn)
        .await?;
    Ok(())
}
