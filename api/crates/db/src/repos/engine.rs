//! The learning engine's storage: candidate features, interest, events, running reel
//! statistics, sessions, colleague shares and the aggregates behind the insights page.

use crate::error::DbError;
use chrono::{DateTime, NaiveDate, Utc};
use reelearn_domain::engine::Preference;
use reelearn_domain::{Channel, EngagementKind};
use sqlx::PgConnection;
use uuid::Uuid;

// ---------------------------------------------------------------- candidates

/// One reel the person may be shown, with everything the value model reads.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct FeatureRow {
    pub reel_id: Uuid,
    pub topic_id: Uuid,
    pub tags: Vec<String>,
    pub duration_ms: i32,
    pub published_at: DateTime<Utc>,
    pub difficulty: Option<i16>,
    pub mandatory: bool,
    pub assigned: bool,
    pub deadline: Option<NaiveDate>,
    pub has_question: bool,
    pub completed_before: bool,
    pub last_completed_at: Option<DateTime<Utc>>,
    pub impressions: i32,
    pub starts: i32,
    pub skips: i32,
    pub completions: i32,
    pub rewatches: i32,
    pub saves: i32,
    pub shares: i32,
    pub not_relevant: i32,
    pub more_like_this: i32,
    pub answers: i32,
    pub answers_correct: i32,
}

/// Which reels to load features for.
pub enum Scope<'a> {
    /// Exactly these reels (the daily plan's candidates).
    Ids(&'a [Uuid]),
    /// Everything the person may see in practice, minus `exclude` and reels finished in the
    /// last `cooldown_days`.
    Practice {
        open_catalog: bool,
        exclude: &'a [Uuid],
        cooldown_days: i32,
    },
}

pub async fn features(
    conn: &mut PgConnection,
    user_id: Uuid,
    day: NaiveDate,
    scope: Scope<'_>,
) -> Result<Vec<FeatureRow>, DbError> {
    let (ids, open_catalog, exclude, cooldown): (Option<Vec<Uuid>>, bool, Vec<Uuid>, i32) =
        match scope {
            Scope::Ids(ids) => (Some(ids.to_vec()), true, vec![], 0),
            Scope::Practice {
                open_catalog,
                exclude,
                cooldown_days,
            } => (None, open_catalog, exclude.to_vec(), cooldown_days),
        };
    Ok(sqlx::query_as::<_, FeatureRow>(
        "WITH assigned AS (
           SELECT r.id AS reel_id, bool_or(a.is_mandatory OR t.is_mandatory) AS mandatory, max(a.ends_on) AS deadline
           FROM topic_assignments a
           JOIN group_members gm ON gm.group_id = a.group_id AND gm.user_id = $1
           JOIN topics t ON t.id = a.topic_id AND t.status = 'active'
           JOIN reels r ON r.topic_id = a.topic_id AND r.status = 'published' AND r.published_at IS NOT NULL
           WHERE a.starts_on <= $2 AND (a.ends_on IS NULL OR a.ends_on >= $2)
           GROUP BY r.id
         ),
         done AS (
           SELECT reel_id, max(started_at) AS last_at FROM watch_events
           WHERE user_id = $1 AND completed GROUP BY reel_id
         )
         SELECT r.id AS reel_id, r.topic_id, r.tags, coalesce(r.duration_ms, 30000) AS duration_ms, r.published_at,
                r.difficulty, coalesce(asg.mandatory, false) AS mandatory, (asg.reel_id IS NOT NULL) AS assigned,
                asg.deadline,
                EXISTS (SELECT 1 FROM questions q WHERE q.reel_id = r.id) AS has_question,
                (done.reel_id IS NOT NULL) AS completed_before, done.last_at AS last_completed_at,
                coalesce(s.impressions, 0) AS impressions, coalesce(s.starts, 0) AS starts, coalesce(s.skips, 0) AS skips,
                coalesce(s.completions, 0) AS completions, coalesce(s.rewatches, 0) AS rewatches,
                coalesce(s.saves, 0) AS saves, coalesce(s.shares, 0) AS shares, coalesce(s.not_relevant, 0) AS not_relevant,
                coalesce(s.more_like_this, 0) AS more_like_this, coalesce(s.answers, 0) AS answers,
                coalesce(s.answers_correct, 0) AS answers_correct
         FROM reels r
         JOIN topics t ON t.id = r.topic_id AND t.status = 'active'
         LEFT JOIN assigned asg ON asg.reel_id = r.id
         LEFT JOIN done ON done.reel_id = r.id
         LEFT JOIN reel_engagement_stats s ON s.reel_id = r.id
         WHERE r.status = 'published' AND r.published_at IS NOT NULL
           AND ($3::uuid[] IS NULL OR r.id = ANY($3))
           AND (asg.reel_id IS NOT NULL OR $4
                OR EXISTS (SELECT 1 FROM reel_shares sh WHERE sh.recipient_id = $1 AND sh.reel_id = r.id))
           AND (NOT EXISTS (SELECT 1 FROM reel_audiences ra WHERE ra.reel_id = r.id)
                OR EXISTS (SELECT 1 FROM reel_audiences ra JOIN group_members gm2 ON gm2.group_id = ra.group_id AND gm2.user_id = $1
                           WHERE ra.reel_id = r.id AND ra.starts_on <= $2))
           AND NOT (r.id = ANY($5))
           AND ($6 = 0 OR done.last_at IS NULL OR done.last_at < now() - make_interval(days => $6))
         ORDER BY r.published_at, r.id",
    )
    .bind(user_id)
    .bind(day)
    .bind(ids)
    .bind(open_catalog)
    .bind(exclude)
    .bind(cooldown)
    .fetch_all(conn)
    .await?)
}

/// Whether the person may play the reel: assigned to them, already in one of their plans,
/// sent to them by a colleague, or in the open catalogue when the company allows it.
pub async fn may_play(
    conn: &mut PgConnection,
    user_id: Uuid,
    reel_id: Uuid,
    open_catalog: bool,
) -> Result<bool, DbError> {
    Ok(sqlx::query_scalar(
        "SELECT EXISTS (
           SELECT 1 FROM reels r JOIN topics t ON t.id = r.topic_id
           WHERE r.id = $2 AND r.status = 'published' AND (
             EXISTS (SELECT 1 FROM topic_assignments a JOIN group_members gm ON gm.group_id = a.group_id WHERE gm.user_id = $1 AND a.topic_id = r.topic_id)
             OR EXISTS (SELECT 1 FROM daily_plan_items i JOIN daily_plans p ON p.id = i.plan_id WHERE p.user_id = $1 AND i.reel_id = r.id)
             OR EXISTS (SELECT 1 FROM reel_shares sh WHERE sh.recipient_id = $1 AND sh.reel_id = r.id)
             OR ($3 AND t.status = 'active' AND (
                   NOT EXISTS (SELECT 1 FROM reel_audiences ra WHERE ra.reel_id = r.id)
                   OR EXISTS (SELECT 1 FROM reel_audiences ra JOIN group_members gm2 ON gm2.group_id = ra.group_id AND gm2.user_id = $1 WHERE ra.reel_id = r.id)))
           ))",
    )
    .bind(user_id)
    .bind(reel_id)
    .bind(open_catalog)
    .fetch_one(conn)
    .await?)
}

// ---------------------------------------------------------------- interest

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TopicInterestRow {
    pub topic_id: Uuid,
    pub score: f32,
    pub events: i32,
    pub last_event_at: DateTime<Utc>,
    pub preference: Option<String>,
}

pub async fn topic_interest(
    conn: &mut PgConnection,
    user_id: Uuid,
) -> Result<Vec<TopicInterestRow>, DbError> {
    Ok(sqlx::query_as::<_, TopicInterestRow>(
        "SELECT topic_id, score, events, last_event_at, preference FROM user_topic_affinity WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_all(conn)
    .await?)
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TagInterestRow {
    pub tag: String,
    pub score: f32,
    pub last_event_at: DateTime<Utc>,
}

pub async fn tag_interest(
    conn: &mut PgConnection,
    user_id: Uuid,
) -> Result<Vec<TagInterestRow>, DbError> {
    Ok(sqlx::query_as::<_, TagInterestRow>(
        "SELECT tag, score, last_event_at FROM user_tag_affinity WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_all(conn)
    .await?)
}

/// Decay the stored topic score to now and add `delta`, in one statement.
pub async fn add_topic_interest(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    user_id: Uuid,
    topic_id: Uuid,
    delta: f32,
    half_life_days: f32,
) -> Result<(), DbError> {
    sqlx::query(
        "INSERT INTO user_topic_affinity (tenant_id, user_id, topic_id, score, events, last_event_at)
         VALUES ($1, $2, $3, greatest(-20, least(50, $4)), 1, now())
         ON CONFLICT (user_id, topic_id) DO UPDATE SET
           score = greatest(-20, least(50,
             user_topic_affinity.score * power(0.5, extract(epoch FROM (now() - user_topic_affinity.last_event_at)) / 86400.0 / $5)
             + $4)),
           events = user_topic_affinity.events + 1,
           last_event_at = now(),
           updated_at = now()",
    )
    .bind(tenant_id)
    .bind(user_id)
    .bind(topic_id)
    .bind(delta)
    .bind(f64::from(half_life_days.max(1.0)))
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn add_tag_interest(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    user_id: Uuid,
    tags: &[String],
    delta: f32,
    half_life_days: f32,
) -> Result<(), DbError> {
    if tags.is_empty() {
        return Ok(());
    }
    sqlx::query(
        "INSERT INTO user_tag_affinity (tenant_id, user_id, tag, score, events, last_event_at)
         SELECT $1, $2, t, greatest(-20, least(50, $4)), 1, now() FROM unnest($3::text[]) AS t
         ON CONFLICT (user_id, tag) DO UPDATE SET
           score = greatest(-20, least(50,
             user_tag_affinity.score * power(0.5, extract(epoch FROM (now() - user_tag_affinity.last_event_at)) / 86400.0 / $5)
             + $4)),
           events = user_tag_affinity.events + 1,
           last_event_at = now()",
    )
    .bind(tenant_id)
    .bind(user_id)
    .bind(tags)
    .bind(delta)
    .bind(f64::from(half_life_days.max(1.0)))
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn set_preference(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    user_id: Uuid,
    topic_id: Uuid,
    preference: Option<Preference>,
) -> Result<(), DbError> {
    sqlx::query(
        "INSERT INTO user_topic_affinity (tenant_id, user_id, topic_id, preference) VALUES ($1, $2, $3, $4)
         ON CONFLICT (user_id, topic_id) DO UPDATE SET preference = EXCLUDED.preference, updated_at = now()",
    )
    .bind(tenant_id)
    .bind(user_id)
    .bind(topic_id)
    .bind(preference.map(|p| p.as_str()))
    .execute(conn)
    .await?;
    Ok(())
}

/// Topics the person can reach (assigned, or every active topic with the open catalogue).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct MyTopicRow {
    pub topic_id: Uuid,
    pub name: String,
    pub name_ar: Option<String>,
    pub is_mandatory: bool,
    pub assigned: bool,
    pub published_reels: i64,
    pub score: Option<f32>,
    pub last_event_at: Option<DateTime<Utc>>,
    pub preference: Option<String>,
}

pub async fn my_topics(
    conn: &mut PgConnection,
    user_id: Uuid,
    open_catalog: bool,
) -> Result<Vec<MyTopicRow>, DbError> {
    Ok(sqlx::query_as::<_, MyTopicRow>(
        "SELECT t.id AS topic_id, t.name, t.name_ar,
                (t.is_mandatory OR EXISTS (SELECT 1 FROM topic_assignments a JOIN group_members gm ON gm.group_id = a.group_id
                                           WHERE gm.user_id = $1 AND a.topic_id = t.id AND a.is_mandatory)) AS is_mandatory,
                EXISTS (SELECT 1 FROM topic_assignments a JOIN group_members gm ON gm.group_id = a.group_id
                        WHERE gm.user_id = $1 AND a.topic_id = t.id) AS assigned,
                (SELECT count(*) FROM reels r WHERE r.topic_id = t.id AND r.status = 'published') AS published_reels,
                ua.score, ua.last_event_at, ua.preference
         FROM topics t
         LEFT JOIN user_topic_affinity ua ON ua.topic_id = t.id AND ua.user_id = $1
         WHERE t.status = 'active'
           AND ($2 OR EXISTS (SELECT 1 FROM topic_assignments a JOIN group_members gm ON gm.group_id = a.group_id
                              WHERE gm.user_id = $1 AND a.topic_id = t.id))
         ORDER BY t.name",
    )
    .bind(user_id)
    .bind(open_catalog)
    .fetch_all(conn)
    .await?)
}

// ---------------------------------------------------------------- events and running stats

#[allow(clippy::too_many_arguments)]
pub async fn insert_event(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    user_id: Uuid,
    reel_id: Uuid,
    session_id: Option<Uuid>,
    kind: EngagementKind,
    watched_ms: Option<i32>,
    position: Option<i16>,
    score: Option<f32>,
    reason: Option<&str>,
    channel: Channel,
) -> Result<(), DbError> {
    sqlx::query(
        "INSERT INTO engagement_events (tenant_id, user_id, reel_id, session_id, kind, watched_ms, position, score, reason, channel)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
    )
    .bind(tenant_id)
    .bind(user_id)
    .bind(reel_id)
    .bind(session_id)
    .bind(kind)
    .bind(watched_ms)
    .bind(position)
    .bind(score)
    .bind(reason)
    .bind(channel)
    .execute(conn)
    .await?;
    Ok(())
}

/// Many impressions in one round trip.
pub async fn insert_impressions(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    user_id: Uuid,
    session_id: Uuid,
    rows: &[(Uuid, i16, f32, String)],
) -> Result<(), DbError> {
    if rows.is_empty() {
        return Ok(());
    }
    let reels: Vec<Uuid> = rows.iter().map(|r| r.0).collect();
    let positions: Vec<i16> = rows.iter().map(|r| r.1).collect();
    let scores: Vec<f32> = rows.iter().map(|r| r.2).collect();
    let reasons: Vec<String> = rows.iter().map(|r| r.3.clone()).collect();
    sqlx::query(
        "INSERT INTO engagement_events (tenant_id, user_id, reel_id, session_id, kind, position, score, reason)
         SELECT $1, $2, x.reel_id, $3, 'impression', x.position, x.score, x.reason
         FROM unnest($4::uuid[], $5::smallint[], $6::real[], $7::text[]) AS x(reel_id, position, score, reason)",
    )
    .bind(tenant_id)
    .bind(user_id)
    .bind(session_id)
    .bind(&reels)
    .bind(&positions)
    .bind(&scores)
    .bind(&reasons)
    .execute(&mut *conn)
    .await?;
    sqlx::query(
        "INSERT INTO reel_engagement_stats (reel_id, tenant_id, impressions)
         SELECT r, $1, 1 FROM unnest($2::uuid[]) AS r
         ON CONFLICT (reel_id) DO UPDATE SET impressions = reel_engagement_stats.impressions + 1, updated_at = now()",
    )
    .bind(tenant_id)
    .bind(&reels)
    .execute(conn)
    .await?;
    Ok(())
}

fn stats_column(kind: EngagementKind) -> &'static str {
    match kind {
        EngagementKind::Impression => "impressions",
        EngagementKind::Start => "starts",
        EngagementKind::Skip => "skips",
        EngagementKind::Complete => "completions",
        EngagementKind::Rewatch => "rewatches",
        EngagementKind::Save => "saves",
        EngagementKind::Unsave => "saves",
        EngagementKind::Share => "shares",
        EngagementKind::NotRelevant => "not_relevant",
        EngagementKind::MoreLikeThis => "more_like_this",
    }
}

/// Bump the running counter for one signal (an unsave takes a save back).
pub async fn bump_reel_stats(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    reel_id: Uuid,
    kind: EngagementKind,
    watched_ms: i64,
) -> Result<(), DbError> {
    let col = stats_column(kind);
    let step = if kind == EngagementKind::Unsave {
        -1
    } else {
        1
    };
    // `col` comes from the fixed match above, never from input.
    let sql = format!(
        "INSERT INTO reel_engagement_stats (reel_id, tenant_id, {col}, watched_ms) VALUES ($1, $2, greatest($3, 0), $4)
         ON CONFLICT (reel_id) DO UPDATE SET {col} = greatest(reel_engagement_stats.{col} + $3, 0),
           watched_ms = reel_engagement_stats.watched_ms + $4, updated_at = now()"
    );
    sqlx::query(&sql)
        .bind(reel_id)
        .bind(tenant_id)
        .bind(step)
        .bind(watched_ms.max(0))
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn bump_reel_answers(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    reel_id: Uuid,
    correct: bool,
) -> Result<(), DbError> {
    sqlx::query(
        "INSERT INTO reel_engagement_stats (reel_id, tenant_id, answers, answers_correct) VALUES ($1, $2, 1, $3)
         ON CONFLICT (reel_id) DO UPDATE SET answers = reel_engagement_stats.answers + 1,
           answers_correct = reel_engagement_stats.answers_correct + $3, updated_at = now()",
    )
    .bind(reel_id)
    .bind(tenant_id)
    .bind(i32::from(correct))
    .execute(conn)
    .await?;
    Ok(())
}

#[derive(Debug, Clone, Copy, sqlx::FromRow)]
pub struct AnswerTotals {
    pub answers: i64,
    pub correct: i64,
}

pub async fn answer_totals(
    conn: &mut PgConnection,
    user_id: Uuid,
) -> Result<AnswerTotals, DbError> {
    Ok(sqlx::query_as::<_, AnswerTotals>(
        "SELECT coalesce(answers_total, 0)::bigint AS answers, coalesce(answers_correct, 0)::bigint AS correct
         FROM (SELECT 1) one LEFT JOIN user_stats us ON us.user_id = $1",
    )
    .bind(user_id)
    .fetch_one(conn)
    .await?)
}

#[derive(Debug, Clone, Copy, sqlx::FromRow)]
pub struct BandRow {
    pub band: i16,
    pub starts: i32,
    pub completions: i32,
    pub skips: i32,
}

pub async fn band_baselines(conn: &mut PgConnection) -> Result<Vec<BandRow>, DbError> {
    Ok(sqlx::query_as::<_, BandRow>(
        "SELECT band, starts, completions, skips FROM duration_band_stats",
    )
    .fetch_all(conn)
    .await?)
}

/// 0..1 per reel: how much the person's location, job title and team engaged this week,
/// relative to the most engaging reel for them.
pub async fn cohort_popularity(
    conn: &mut PgConnection,
    location: Option<&str>,
    job_title: Option<&str>,
    manager_id: Option<Uuid>,
) -> Result<Vec<(Uuid, f32)>, DbError> {
    let rows: Vec<(Uuid, f64)> = sqlx::query_as(
        "SELECT reel_id, sum(completions_7d + 2 * saves_7d + 3 * shares_7d)::float8 AS points
         FROM cohort_reel_stats
         WHERE (cohort_kind = 'location' AND cohort_key = $1)
            OR (cohort_kind = 'job_title' AND cohort_key = $2)
            OR (cohort_kind = 'team' AND cohort_key = $3)
         GROUP BY reel_id",
    )
    .bind(location)
    .bind(job_title)
    .bind(manager_id.map(|m| m.to_string()))
    .fetch_all(conn)
    .await?;
    let max = rows.iter().map(|r| r.1).fold(0.0_f64, f64::max);
    Ok(rows
        .into_iter()
        .map(|(id, p)| (id, if max > 0.0 { (p / max) as f32 } else { 0.0 }))
        .collect())
}

// ---------------------------------------------------------------- sessions

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SessionRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub started_at: DateTime<Utc>,
    pub last_active_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub items_served: i32,
    pub items_completed: i32,
    pub active_ms: i64,
    pub stop_reason: Option<String>,
}

const SCOLS: &str = "id, user_id, started_at, last_active_at, ended_at, items_served, items_completed, active_ms, stop_reason";

pub async fn session(conn: &mut PgConnection, id: Uuid) -> Result<Option<SessionRow>, DbError> {
    Ok(sqlx::query_as::<_, SessionRow>(&format!(
        "SELECT {SCOLS} FROM engine_sessions WHERE id = $1"
    ))
    .bind(id)
    .fetch_optional(conn)
    .await?)
}

pub async fn create_session(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    user_id: Uuid,
) -> Result<SessionRow, DbError> {
    Ok(sqlx::query_as::<_, SessionRow>(&format!(
        "INSERT INTO engine_sessions (tenant_id, user_id) VALUES ($1, $2) RETURNING {SCOLS}"
    ))
    .bind(tenant_id)
    .bind(user_id)
    .fetch_one(conn)
    .await?)
}

pub async fn touch_session(
    conn: &mut PgConnection,
    id: Uuid,
    served: i32,
    completed: i32,
    active_ms: i64,
) -> Result<(), DbError> {
    sqlx::query(
        "UPDATE engine_sessions SET items_served = items_served + $2, items_completed = items_completed + $3,
           active_ms = active_ms + $4, last_active_at = now() WHERE id = $1",
    )
    .bind(id)
    .bind(served)
    .bind(completed)
    .bind(active_ms.max(0))
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn end_session(conn: &mut PgConnection, id: Uuid, reason: &str) -> Result<(), DbError> {
    sqlx::query("UPDATE engine_sessions SET ended_at = coalesce(ended_at, now()), stop_reason = coalesce(stop_reason, $2) WHERE id = $1")
        .bind(id)
        .bind(reason)
        .execute(conn)
        .await?;
    Ok(())
}

/// Practice time since `since` (the start of the person's local day).
pub async fn practice_ms_since(
    conn: &mut PgConnection,
    user_id: Uuid,
    since: DateTime<Utc>,
) -> Result<i64, DbError> {
    Ok(sqlx::query_scalar::<_, Option<i64>>(
        "SELECT sum(active_ms)::bigint FROM engine_sessions WHERE user_id = $1 AND last_active_at >= $2",
    )
    .bind(user_id)
    .bind(since)
    .fetch_one(conn)
    .await?
    .unwrap_or(0))
}

pub async fn session_reels(
    conn: &mut PgConnection,
    session_id: Uuid,
) -> Result<Vec<Uuid>, DbError> {
    Ok(sqlx::query_scalar(
        "SELECT DISTINCT reel_id FROM engagement_events WHERE session_id = $1 AND kind = 'impression'",
    )
    .bind(session_id)
    .fetch_all(conn)
    .await?)
}

// ---------------------------------------------------------------- reviews for practice

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct WeakMemoryRow {
    pub question_id: Uuid,
    pub reel_id: Uuid,
    pub due_on: NaiveDate,
    pub interval_days: i16,
    pub ease: f32,
    pub reps: i16,
    pub lapses: i16,
    pub last_result: Option<bool>,
}

impl From<&WeakMemoryRow> for reelearn_domain::recall::MemoryState {
    fn from(r: &WeakMemoryRow) -> Self {
        reelearn_domain::recall::MemoryState {
            due_on: r.due_on,
            interval_days: r.interval_days,
            ease: r.ease,
            reps: r.reps,
            lapses: r.lapses,
            last_result: r.last_result,
        }
    }
}

/// Questions worth a look in practice: wrong last time, or due within three days.
pub async fn weak_memories(
    conn: &mut PgConnection,
    user_id: Uuid,
    day: NaiveDate,
    exclude_questions: &[Uuid],
    limit: i64,
) -> Result<Vec<WeakMemoryRow>, DbError> {
    Ok(sqlx::query_as::<_, WeakMemoryRow>(
        "SELECT m.question_id, q.reel_id, m.due_on, m.interval_days, m.ease, m.reps, m.lapses, m.last_result
         FROM memory_states m JOIN questions q ON q.id = m.question_id
         JOIN reels r ON r.id = q.reel_id AND r.status = 'published'
         WHERE m.user_id = $1 AND (m.last_result = false OR m.due_on <= $2 + 3)
           AND NOT (m.question_id = ANY($3))
           AND (m.last_answered_at IS NULL OR m.last_answered_at < now() - interval '6 hours')
         ORDER BY (m.last_result = false) DESC, m.due_on, m.question_id
         LIMIT $4",
    )
    .bind(user_id)
    .bind(day)
    .bind(exclude_questions)
    .bind(limit)
    .fetch_all(conn)
    .await?)
}

// ---------------------------------------------------------------- sharing

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct InboxRow {
    pub reel_id: Uuid,
    pub sender_name: String,
    pub note: Option<String>,
}

pub async fn share_inbox(conn: &mut PgConnection, user_id: Uuid) -> Result<Vec<InboxRow>, DbError> {
    Ok(sqlx::query_as::<_, InboxRow>(
        "SELECT DISTINCT ON (s.reel_id) s.reel_id, u.full_name AS sender_name, s.note
         FROM reel_shares s JOIN users u ON u.id = s.sender_id
         WHERE s.recipient_id = $1 AND s.opened_at IS NULL
         ORDER BY s.reel_id, s.created_at DESC",
    )
    .bind(user_id)
    .fetch_all(conn)
    .await?)
}

pub async fn create_share(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    reel_id: Uuid,
    sender_id: Uuid,
    recipient_id: Uuid,
    note: Option<&str>,
) -> Result<bool, DbError> {
    let r = sqlx::query(
        "INSERT INTO reel_shares (tenant_id, reel_id, sender_id, recipient_id, note) VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (reel_id, sender_id, recipient_id) DO NOTHING",
    )
    .bind(tenant_id)
    .bind(reel_id)
    .bind(sender_id)
    .bind(recipient_id)
    .bind(note)
    .execute(conn)
    .await?;
    Ok(r.rows_affected() == 1)
}

pub async fn shares_sent_since(
    conn: &mut PgConnection,
    sender_id: Uuid,
    since: DateTime<Utc>,
) -> Result<i64, DbError> {
    Ok(sqlx::query_scalar(
        "SELECT count(*) FROM reel_shares WHERE sender_id = $1 AND created_at >= $2",
    )
    .bind(sender_id)
    .bind(since)
    .fetch_one(conn)
    .await?)
}

pub async fn mark_shares_opened(
    conn: &mut PgConnection,
    recipient_id: Uuid,
    reel_id: Uuid,
) -> Result<(), DbError> {
    sqlx::query("UPDATE reel_shares SET opened_at = now() WHERE recipient_id = $1 AND reel_id = $2 AND opened_at IS NULL")
        .bind(recipient_id)
        .bind(reel_id)
        .execute(conn)
        .await?;
    Ok(())
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ColleagueRow {
    pub id: Uuid,
    pub full_name: String,
    pub job_title: Option<String>,
    pub location: Option<String>,
    pub strength: i64,
}

/// Active colleagues, strongest connection first: past shares either way, same manager,
/// same location, same job title.
pub async fn colleagues(
    conn: &mut PgConnection,
    user_id: Uuid,
    q: Option<&str>,
    limit: i64,
) -> Result<Vec<ColleagueRow>, DbError> {
    Ok(sqlx::query_as::<_, ColleagueRow>(
        "WITH me AS (SELECT manager_id, location, job_title FROM users WHERE id = $1)
         SELECT u.id, u.full_name, u.job_title, u.location,
                (3 * (SELECT count(*) FROM reel_shares s WHERE (s.sender_id = $1 AND s.recipient_id = u.id) OR (s.sender_id = u.id AND s.recipient_id = $1))
                 + CASE WHEN u.manager_id IS NOT DISTINCT FROM me.manager_id AND me.manager_id IS NOT NULL THEN 2 ELSE 0 END
                 + CASE WHEN u.id = me.manager_id OR u.manager_id = $1 THEN 2 ELSE 0 END
                 + CASE WHEN u.location IS NOT DISTINCT FROM me.location AND me.location IS NOT NULL THEN 1 ELSE 0 END
                 + CASE WHEN u.job_title IS NOT DISTINCT FROM me.job_title AND me.job_title IS NOT NULL THEN 1 ELSE 0 END)::bigint AS strength
         FROM users u, me
         WHERE u.id <> $1 AND u.status = 'active'
           AND ($2::text IS NULL OR u.full_name ILIKE '%' || $2 || '%')
         ORDER BY strength DESC, u.full_name
         LIMIT $3",
    )
    .bind(user_id)
    .bind(q.map(str::trim).filter(|s| !s.is_empty()))
    .bind(limit)
    .fetch_all(conn)
    .await?)
}

// ---------------------------------------------------------------- aggregates (worker)

/// Recompute the per-band completion baselines from the running reel statistics.
pub async fn refresh_band_stats(conn: &mut PgConnection, tenant_id: Uuid) -> Result<u64, DbError> {
    let r = sqlx::query(
        "INSERT INTO duration_band_stats (tenant_id, band, starts, completions, skips, refreshed_at)
         SELECT $1, b.band, coalesce(sum(s.starts), 0), coalesce(sum(s.completions), 0), coalesce(sum(s.skips), 0), now()
         FROM generate_series(0, 4) AS b(band)
         LEFT JOIN reels r ON r.tenant_id = $1 AND (CASE
              WHEN coalesce(r.duration_ms, 30000) < 20000 THEN 0
              WHEN coalesce(r.duration_ms, 30000) < 35000 THEN 1
              WHEN coalesce(r.duration_ms, 30000) < 50000 THEN 2
              WHEN coalesce(r.duration_ms, 30000) < 70000 THEN 3
              ELSE 4 END) = b.band
         LEFT JOIN reel_engagement_stats s ON s.reel_id = r.id
         GROUP BY b.band
         ON CONFLICT (tenant_id, band) DO UPDATE SET starts = EXCLUDED.starts, completions = EXCLUDED.completions,
           skips = EXCLUDED.skips, refreshed_at = now()",
    )
    .bind(tenant_id)
    .execute(conn)
    .await?;
    Ok(r.rows_affected())
}

/// Reels created before tagging existed get their topic as a first tag, so tag interest
/// covers the whole catalogue. Generated reels arrive tagged by the studio.
pub async fn backfill_topic_tags(conn: &mut PgConnection, tenant_id: Uuid) -> Result<u64, DbError> {
    let r = sqlx::query(
        "UPDATE reels r SET tags = ARRAY[lower(t.name)] FROM topics t
         WHERE t.id = r.topic_id AND r.tenant_id = $1 AND r.tags = '{}'",
    )
    .bind(tenant_id)
    .execute(conn)
    .await?;
    Ok(r.rows_affected())
}

/// Rebuild the 7-day cohort table (location, job title, team) from the event log.
pub async fn refresh_cohort_stats(
    conn: &mut PgConnection,
    tenant_id: Uuid,
) -> Result<u64, DbError> {
    sqlx::query("DELETE FROM cohort_reel_stats WHERE tenant_id = $1")
        .bind(tenant_id)
        .execute(&mut *conn)
        .await?;
    let r = sqlx::query(
        "INSERT INTO cohort_reel_stats (tenant_id, cohort_kind, cohort_key, reel_id, completions_7d, saves_7d, shares_7d, learners_7d)
         SELECT $1, c.kind, c.key, e.reel_id,
                count(*) FILTER (WHERE e.kind = 'complete'), count(*) FILTER (WHERE e.kind = 'save'),
                count(*) FILTER (WHERE e.kind = 'share'), count(DISTINCT e.user_id)
         FROM engagement_events e
         JOIN users u ON u.id = e.user_id
         CROSS JOIN LATERAL (VALUES ('location', u.location), ('job_title', u.job_title), ('team', u.manager_id::text)) AS c(kind, key)
         WHERE e.tenant_id = $1 AND e.occurred_at >= now() - interval '7 days'
           AND e.kind IN ('complete', 'save', 'share') AND c.key IS NOT NULL
         GROUP BY c.kind, c.key, e.reel_id",
    )
    .bind(tenant_id)
    .execute(conn)
    .await?;
    Ok(r.rows_affected())
}

/// People who should get a reminder now: reminders on, active, not paused, away longer than
/// `away_hours`, today's plan not done, and fewer than `max_per_day` reminders since `day_start`.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ReengageRow {
    pub user_id: Uuid,
    pub channel: Channel,
}

pub async fn reengage_candidates(
    conn: &mut PgConnection,
    day: NaiveDate,
    day_start: DateTime<Utc>,
    away_hours: i32,
    max_per_day: i64,
    limit: i64,
) -> Result<Vec<ReengageRow>, DbError> {
    Ok(sqlx::query_as::<_, ReengageRow>(
        "SELECT u.id AS user_id, u.delivery_channel AS channel FROM users u
         WHERE u.status = 'active' AND u.role = 'learner' AND u.reminders_on
           AND (u.paused_until IS NULL OR u.paused_until < $1)
           AND (u.last_seen_at IS NULL OR u.last_seen_at < now() - make_interval(hours => $3))
           AND EXISTS (SELECT 1 FROM daily_plans p WHERE p.user_id = u.id AND p.plan_date = $1 AND p.item_count > 0 AND p.status <> 'done')
           AND (SELECT count(*) FROM deliveries d WHERE d.user_id = u.id AND d.kind IN ('reminder', 'nudge')
                AND d.scheduled_for >= $2) < $4
           AND NOT EXISTS (SELECT 1 FROM deliveries d WHERE d.user_id = u.id AND d.kind IN ('reminder', 'nudge')
                AND d.scheduled_for >= now() - make_interval(hours => greatest($3, 1)))
         ORDER BY u.last_seen_at NULLS FIRST
         LIMIT $5",
    )
    .bind(day)
    .bind(day_start)
    .bind(away_hours)
    .bind(max_per_day)
    .bind(limit)
    .fetch_all(conn)
    .await?)
}

// ---------------------------------------------------------------- insights (admin)

#[derive(Debug, Clone, Copy, Default, sqlx::FromRow)]
pub struct SessionTotals {
    pub sessions: i64,
    pub learners: i64,
    pub active_ms: i64,
    pub served: i64,
}

pub async fn session_totals(
    conn: &mut PgConnection,
    since: DateTime<Utc>,
) -> Result<SessionTotals, DbError> {
    Ok(sqlx::query_as::<_, SessionTotals>(
        "SELECT count(*)::bigint AS sessions, count(DISTINCT user_id)::bigint AS learners,
                coalesce(sum(active_ms), 0)::bigint AS active_ms, coalesce(sum(items_served), 0)::bigint AS served
         FROM engine_sessions WHERE started_at >= $1",
    )
    .bind(since)
    .fetch_one(conn)
    .await?)
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct KindCount {
    pub kind: String,
    pub events: i64,
    pub learners: i64,
}

pub async fn event_counts(
    conn: &mut PgConnection,
    since: DateTime<Utc>,
) -> Result<Vec<KindCount>, DbError> {
    Ok(sqlx::query_as::<_, KindCount>(
        "SELECT kind::text AS kind, count(*)::bigint AS events, count(DISTINCT user_id)::bigint AS learners
         FROM engagement_events WHERE occurred_at >= $1 GROUP BY kind",
    )
    .bind(since)
    .fetch_all(conn)
    .await?)
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct DayRow {
    pub day: NaiveDate,
    pub sessions: i64,
    pub active_ms: i64,
    pub starts: i64,
    pub completions: i64,
}

pub async fn daily(
    conn: &mut PgConnection,
    since: DateTime<Utc>,
    timezone: &str,
) -> Result<Vec<DayRow>, DbError> {
    Ok(sqlx::query_as::<_, DayRow>(
        "WITH s AS (
           SELECT (started_at AT TIME ZONE $2)::date AS day, count(*)::bigint AS sessions, coalesce(sum(active_ms), 0)::bigint AS active_ms
           FROM engine_sessions WHERE started_at >= $1 GROUP BY 1
         ), e AS (
           SELECT (occurred_at AT TIME ZONE $2)::date AS day,
                  count(*) FILTER (WHERE kind = 'start')::bigint AS starts,
                  count(*) FILTER (WHERE kind = 'complete')::bigint AS completions
           FROM engagement_events WHERE occurred_at >= $1 AND kind IN ('start', 'complete') GROUP BY 1
         )
         SELECT coalesce(s.day, e.day) AS day, coalesce(s.sessions, 0) AS sessions, coalesce(s.active_ms, 0) AS active_ms,
                coalesce(e.starts, 0) AS starts, coalesce(e.completions, 0) AS completions
         FROM s FULL JOIN e ON e.day = s.day ORDER BY 1",
    )
    .bind(since)
    .bind(timezone)
    .fetch_all(conn)
    .await?)
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TopicInterestAgg {
    pub topic_id: Uuid,
    pub name: String,
    pub interested: i64,
    pub followers: i64,
    pub muted: i64,
    pub avg_score: f64,
}

pub async fn topic_interest_agg(
    conn: &mut PgConnection,
    limit: i64,
) -> Result<Vec<TopicInterestAgg>, DbError> {
    Ok(sqlx::query_as::<_, TopicInterestAgg>(
        "SELECT t.id AS topic_id, t.name,
                count(ua.user_id) FILTER (WHERE ua.score > 2)::bigint AS interested,
                count(ua.user_id) FILTER (WHERE ua.preference = 'follow')::bigint AS followers,
                count(ua.user_id) FILTER (WHERE ua.preference = 'mute')::bigint AS muted,
                coalesce(avg(ua.score), 0)::float8 AS avg_score
         FROM topics t LEFT JOIN user_topic_affinity ua ON ua.topic_id = t.id
         WHERE t.status = 'active'
         GROUP BY t.id, t.name
         ORDER BY interested DESC, followers DESC, t.name
         LIMIT $1",
    )
    .bind(limit)
    .fetch_all(conn)
    .await?)
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TagInterestAgg {
    pub tag: String,
    pub interested: i64,
    pub avg_score: f64,
}

pub async fn tag_interest_agg(
    conn: &mut PgConnection,
    limit: i64,
) -> Result<Vec<TagInterestAgg>, DbError> {
    Ok(sqlx::query_as::<_, TagInterestAgg>(
        "SELECT tag, count(*) FILTER (WHERE score > 2)::bigint AS interested, avg(score)::float8 AS avg_score
         FROM user_tag_affinity GROUP BY tag ORDER BY interested DESC, avg_score DESC, tag LIMIT $1",
    )
    .bind(limit)
    .fetch_all(conn)
    .await?)
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CohortRow {
    pub key: String,
    pub learners: i64,
    pub active_ms: i64,
    pub sessions: i64,
}

pub async fn cohort_practice(
    conn: &mut PgConnection,
    since: DateTime<Utc>,
    limit: i64,
) -> Result<Vec<CohortRow>, DbError> {
    Ok(sqlx::query_as::<_, CohortRow>(
        "SELECT coalesce(u.location, 'Unassigned') AS key, count(DISTINCT s.user_id)::bigint AS learners,
                coalesce(sum(s.active_ms), 0)::bigint AS active_ms, count(*)::bigint AS sessions
         FROM engine_sessions s JOIN users u ON u.id = s.user_id
         WHERE s.started_at >= $1 GROUP BY 1 ORDER BY active_ms DESC, key LIMIT $2",
    )
    .bind(since)
    .bind(limit)
    .fetch_all(conn)
    .await?)
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ReasonRow {
    pub reason: String,
    pub impressions: i64,
}

pub async fn reason_mix(
    conn: &mut PgConnection,
    since: DateTime<Utc>,
) -> Result<Vec<ReasonRow>, DbError> {
    Ok(sqlx::query_as::<_, ReasonRow>(
        "SELECT coalesce(reason, 'unknown') AS reason, count(*)::bigint AS impressions FROM engagement_events
         WHERE kind = 'impression' AND occurred_at >= $1 GROUP BY 1 ORDER BY 2 DESC",
    )
    .bind(since)
    .fetch_all(conn)
    .await?)
}

#[derive(Debug, Clone, Copy, Default, sqlx::FromRow)]
pub struct ShareTotals {
    pub shares: i64,
    pub senders: i64,
    pub recipients: i64,
    pub opened: i64,
}

pub async fn share_totals(
    conn: &mut PgConnection,
    since: DateTime<Utc>,
) -> Result<ShareTotals, DbError> {
    Ok(sqlx::query_as::<_, ShareTotals>(
        "SELECT count(*)::bigint AS shares, count(DISTINCT sender_id)::bigint AS senders,
                count(DISTINCT recipient_id)::bigint AS recipients, count(*) FILTER (WHERE opened_at IS NOT NULL)::bigint AS opened
         FROM reel_shares WHERE created_at >= $1",
    )
    .bind(since)
    .fetch_one(conn)
    .await?)
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ReelPerfRow {
    pub reel_id: Uuid,
    pub title: String,
    pub topic_name: String,
    pub duration_ms: i32,
    pub impressions: i32,
    pub starts: i32,
    pub completions: i32,
    pub skips: i32,
    pub saves: i32,
    pub shares: i32,
    pub not_relevant: i32,
    pub answers: i32,
    pub answers_correct: i32,
}

pub async fn reel_performance(
    conn: &mut PgConnection,
    limit: i64,
) -> Result<Vec<ReelPerfRow>, DbError> {
    Ok(sqlx::query_as::<_, ReelPerfRow>(
        "SELECT r.id AS reel_id, r.title, t.name AS topic_name, coalesce(r.duration_ms, 30000) AS duration_ms,
                s.impressions, s.starts, s.completions, s.skips, s.saves, s.shares, s.not_relevant, s.answers, s.answers_correct
         FROM reel_engagement_stats s JOIN reels r ON r.id = s.reel_id JOIN topics t ON t.id = r.topic_id
         ORDER BY s.starts DESC, s.impressions DESC, r.title LIMIT $1",
    )
    .bind(limit)
    .fetch_all(conn)
    .await?)
}
