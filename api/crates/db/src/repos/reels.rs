use crate::error::DbError;
use crate::repos::push_cursor_filter;
use chrono::{DateTime, NaiveDate, Utc};
use reelearn_domain::cursor::Cursor;
use reelearn_domain::{ApprovalRole, Decision, ExtractionStatus, ReelKind, ReelStatus};
use sqlx::{PgConnection, QueryBuilder};
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ReelRow {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub topic_id: Uuid,
    pub source_id: Option<Uuid>,
    pub generation_job_id: Option<Uuid>,
    pub kind: ReelKind,
    pub status: ReelStatus,
    pub title: String,
    pub language: String,
    pub script: Option<String>,
    pub source_citation: Option<String>,
    pub presenter: serde_json::Value,
    pub video_key: Option<String>,
    pub thumbnail_key: Option<String>,
    pub captions_key: Option<String>,
    pub duration_ms: Option<i32>,
    pub version: i32,
    pub compliance_tag: Option<String>,
    pub consent_confirmed: bool,
    pub created_by: Uuid,
    pub published_at: Option<DateTime<Utc>>,
    pub archived_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ReelRow {
    pub fn presenter_name(&self) -> Option<String> {
        self.presenter
            .get("name")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }
}

const COLS: &str = "r.id, r.tenant_id, r.topic_id, r.source_id, r.generation_job_id, r.kind, r.status, r.title, r.language, r.script, r.source_citation, r.presenter, r.video_key, r.thumbnail_key, r.captions_key, r.duration_ms, r.version, r.compliance_tag, r.consent_confirmed, r.created_by, r.published_at, r.archived_at, r.created_at, r.updated_at";

pub async fn get(conn: &mut PgConnection, id: Uuid) -> Result<ReelRow, DbError> {
    find(conn, id).await?.ok_or(DbError::NotFound("reel"))
}

pub async fn find(conn: &mut PgConnection, id: Uuid) -> Result<Option<ReelRow>, DbError> {
    Ok(
        sqlx::query_as::<_, ReelRow>(&format!("SELECT {COLS} FROM reels r WHERE r.id = $1"))
            .bind(id)
            .fetch_optional(conn)
            .await?,
    )
}

pub async fn get_for_update(conn: &mut PgConnection, id: Uuid) -> Result<ReelRow, DbError> {
    sqlx::query_as::<_, ReelRow>(&format!(
        "SELECT {COLS} FROM reels r WHERE r.id = $1 FOR UPDATE"
    ))
    .bind(id)
    .fetch_optional(conn)
    .await?
    .ok_or(DbError::NotFound("reel"))
}

pub async fn by_ids(conn: &mut PgConnection, ids: &[Uuid]) -> Result<Vec<ReelRow>, DbError> {
    Ok(
        sqlx::query_as::<_, ReelRow>(&format!("SELECT {COLS} FROM reels r WHERE r.id = ANY($1)"))
            .bind(ids)
            .fetch_all(conn)
            .await?,
    )
}

pub async fn for_job(conn: &mut PgConnection, job_id: Uuid) -> Result<Vec<ReelRow>, DbError> {
    Ok(sqlx::query_as::<_, ReelRow>(&format!(
        "SELECT {COLS} FROM reels r WHERE r.generation_job_id = $1 ORDER BY r.created_at, r.id"
    ))
    .bind(job_id)
    .fetch_all(conn)
    .await?)
}

#[derive(Debug, Default, Clone)]
pub struct ReelFilter {
    pub status: Option<ReelStatus>,
    pub topic_id: Option<Uuid>,
    pub language: Option<String>,
    pub kind: Option<ReelKind>,
    pub q: Option<String>,
}

pub async fn list(
    conn: &mut PgConnection,
    f: &ReelFilter,
    cursor: Option<Cursor>,
    limit: i64,
) -> Result<Vec<ReelRow>, DbError> {
    let mut qb: QueryBuilder<sqlx::Postgres> =
        QueryBuilder::new(format!("SELECT {COLS} FROM reels r WHERE true"));
    if let Some(s) = f.status {
        qb.push(" AND r.status = ");
        qb.push_bind(s);
    }
    if let Some(t) = f.topic_id {
        qb.push(" AND r.topic_id = ");
        qb.push_bind(t);
    }
    if let Some(l) = f.language.as_deref() {
        qb.push(" AND (r.language = ");
        qb.push_bind(l.to_string());
        qb.push(
            " OR EXISTS (SELECT 1 FROM reel_variants v WHERE v.reel_id = r.id AND v.language = ",
        );
        qb.push_bind(l.to_string());
        qb.push("))");
    }
    if let Some(k) = f.kind {
        qb.push(" AND r.kind = ");
        qb.push_bind(k);
    }
    if let Some(q) = f.q.as_deref().filter(|s| !s.trim().is_empty()) {
        qb.push(" AND (to_tsvector('simple', r.title || ' ' || coalesce(r.script, '')) @@ plainto_tsquery('simple', ");
        qb.push_bind(q.trim().to_string());
        qb.push(") OR r.title ILIKE ");
        qb.push_bind(format!("%{}%", q.trim()));
        qb.push(")");
    }
    push_cursor_filter(&mut qb, "r.updated_at", "r.id", cursor);
    qb.push(" ORDER BY r.updated_at DESC, r.id DESC LIMIT ");
    qb.push_bind(limit + 1);
    Ok(qb.build_query_as::<ReelRow>().fetch_all(conn).await?)
}

pub struct NewReel<'a> {
    pub topic_id: Uuid,
    pub source_id: Option<Uuid>,
    pub generation_job_id: Option<Uuid>,
    pub kind: ReelKind,
    pub status: ReelStatus,
    pub title: &'a str,
    pub language: &'a str,
    pub script: Option<&'a str>,
    pub source_citation: Option<&'a str>,
    pub presenter: serde_json::Value,
    pub video_key: Option<&'a str>,
    pub captions_key: Option<&'a str>,
    pub thumbnail_key: Option<&'a str>,
    pub duration_ms: Option<i32>,
    pub compliance_tag: Option<&'a str>,
    pub consent_confirmed: bool,
    pub created_by: Uuid,
}

pub async fn create(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    r: NewReel<'_>,
) -> Result<ReelRow, DbError> {
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO reels (tenant_id, topic_id, source_id, generation_job_id, kind, status, title, language, script, source_citation, presenter, video_key, captions_key, thumbnail_key, duration_ms, compliance_tag, consent_confirmed, created_by)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18) RETURNING id",
    )
    .bind(tenant_id)
    .bind(r.topic_id)
    .bind(r.source_id)
    .bind(r.generation_job_id)
    .bind(r.kind)
    .bind(r.status)
    .bind(r.title)
    .bind(r.language)
    .bind(r.script)
    .bind(r.source_citation)
    .bind(r.presenter)
    .bind(r.video_key)
    .bind(r.captions_key)
    .bind(r.thumbnail_key)
    .bind(r.duration_ms)
    .bind(r.compliance_tag)
    .bind(r.consent_confirmed)
    .bind(r.created_by)
    .fetch_one(&mut *conn)
    .await?;
    get(conn, id).await
}

#[derive(Debug, Default, Clone)]
pub struct ReelUpdate {
    pub title: Option<String>,
    pub script: Option<String>,
    pub source_citation: Option<String>,
    pub presenter: Option<serde_json::Value>,
    pub compliance_tag: Option<String>,
    pub video_key: Option<String>,
    pub thumbnail_key: Option<String>,
    pub captions_key: Option<String>,
    pub duration_ms: Option<i32>,
    pub consent_confirmed: Option<bool>,
}

pub async fn update(conn: &mut PgConnection, id: Uuid, p: ReelUpdate) -> Result<ReelRow, DbError> {
    sqlx::query(
        "UPDATE reels SET title = coalesce($2, title), script = coalesce($3, script), source_citation = coalesce($4, source_citation),
         presenter = coalesce($5, presenter), compliance_tag = coalesce($6, compliance_tag), video_key = coalesce($7, video_key),
         thumbnail_key = coalesce($8, thumbnail_key), captions_key = coalesce($9, captions_key), duration_ms = coalesce($10, duration_ms),
         consent_confirmed = coalesce($11, consent_confirmed)
         WHERE id = $1",
    )
    .bind(id)
    .bind(p.title)
    .bind(p.script)
    .bind(p.source_citation)
    .bind(p.presenter)
    .bind(p.compliance_tag)
    .bind(p.video_key)
    .bind(p.thumbnail_key)
    .bind(p.captions_key)
    .bind(p.duration_ms)
    .bind(p.consent_confirmed)
    .execute(&mut *conn)
    .await?;
    get(conn, id).await
}

pub async fn set_status(
    conn: &mut PgConnection,
    id: Uuid,
    status: ReelStatus,
) -> Result<ReelRow, DbError> {
    sqlx::query("UPDATE reels SET status = $2, archived_at = CASE WHEN $2 = 'archived'::reel_status_t THEN now() ELSE archived_at END WHERE id = $1")
        .bind(id)
        .bind(status)
        .execute(&mut *conn)
        .await?;
    get(conn, id).await
}

/// Editing a published reel: new version, back to review. Learners keep their memory states.
pub async fn bump_version_for_review(
    conn: &mut PgConnection,
    id: Uuid,
) -> Result<ReelRow, DbError> {
    sqlx::query("UPDATE reels SET version = version + 1, status = 'in_review', published_at = NULL WHERE id = $1").bind(id).execute(&mut *conn).await?;
    get(conn, id).await
}

/// Publish; the `reels_require_dual_approval` trigger enforces both sign-offs on this version.
pub async fn publish(conn: &mut PgConnection, id: Uuid) -> Result<ReelRow, DbError> {
    sqlx::query("UPDATE reels SET status = 'published' WHERE id = $1")
        .bind(id)
        .execute(&mut *conn)
        .await?;
    get(conn, id).await
}

pub async fn approved_roles_for_version(
    conn: &mut PgConnection,
    reel_id: Uuid,
    version: i32,
) -> Result<Vec<ApprovalRole>, DbError> {
    Ok(sqlx::query_scalar("SELECT DISTINCT role FROM approvals WHERE reel_id = $1 AND decision = 'approved' AND reel_version = $2")
        .bind(reel_id)
        .bind(version)
        .fetch_all(conn)
        .await?)
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ApprovalRow {
    pub id: Uuid,
    pub role: ApprovalRole,
    pub decision: Decision,
    pub reviewer_id: Uuid,
    pub reviewer_name: String,
    pub comment: Option<String>,
    pub reel_version: i32,
    pub decided_at: DateTime<Utc>,
}

pub async fn approvals(
    conn: &mut PgConnection,
    reel_id: Uuid,
) -> Result<Vec<ApprovalRow>, DbError> {
    Ok(sqlx::query_as::<_, ApprovalRow>(
        "SELECT a.id, a.role, a.decision, a.reviewer_id, u.full_name AS reviewer_name, a.comment, a.reel_version, a.decided_at
         FROM approvals a JOIN users u ON u.id = a.reviewer_id WHERE a.reel_id = $1 ORDER BY a.decided_at DESC",
    )
    .bind(reel_id)
    .fetch_all(conn)
    .await?)
}

pub async fn add_approval(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    reel_id: Uuid,
    reviewer_id: Uuid,
    role: ApprovalRole,
    decision: Decision,
    comment: Option<&str>,
    version: i32,
) -> Result<(), DbError> {
    sqlx::query("INSERT INTO approvals (tenant_id, reel_id, reviewer_id, role, decision, comment, reel_version) VALUES ($1, $2, $3, $4, $5, $6, $7)")
        .bind(tenant_id)
        .bind(reel_id)
        .bind(reviewer_id)
        .bind(role)
        .bind(decision)
        .bind(comment)
        .bind(version)
        .execute(conn)
        .await?;
    Ok(())
}

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct AudienceRow {
    pub id: Uuid,
    pub name: String,
}

pub async fn audience(conn: &mut PgConnection, reel_id: Uuid) -> Result<Vec<AudienceRow>, DbError> {
    Ok(sqlx::query_as::<_, AudienceRow>("SELECT g.id, g.name FROM reel_audiences ra JOIN groups g ON g.id = ra.group_id WHERE ra.reel_id = $1 ORDER BY g.name")
        .bind(reel_id)
        .fetch_all(conn)
        .await?)
}

pub async fn set_audience(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    reel_id: Uuid,
    group_ids: &[Uuid],
    starts_on: Option<NaiveDate>,
) -> Result<(), DbError> {
    sqlx::query("DELETE FROM reel_audiences WHERE reel_id = $1")
        .bind(reel_id)
        .execute(&mut *conn)
        .await?;
    for g in group_ids {
        sqlx::query("INSERT INTO reel_audiences (reel_id, group_id, tenant_id, starts_on) VALUES ($1, $2, $3, coalesce($4, CURRENT_DATE)) ON CONFLICT DO NOTHING")
            .bind(reel_id)
            .bind(g)
            .bind(tenant_id)
            .bind(starts_on)
            .execute(&mut *conn)
            .await?;
    }
    Ok(())
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct VariantRow {
    pub id: Uuid,
    pub reel_id: Uuid,
    pub language: String,
    pub video_key: Option<String>,
    pub captions_key: Option<String>,
    pub audio_key: Option<String>,
    pub script: Option<String>,
    pub dub_status: ExtractionStatus,
}

const VCOLS: &str = "id, reel_id, language, video_key, captions_key, audio_key, script, dub_status";

pub async fn variants(conn: &mut PgConnection, reel_id: Uuid) -> Result<Vec<VariantRow>, DbError> {
    Ok(sqlx::query_as::<_, VariantRow>(&format!(
        "SELECT {VCOLS} FROM reel_variants WHERE reel_id = $1 ORDER BY language"
    ))
    .bind(reel_id)
    .fetch_all(conn)
    .await?)
}

pub async fn variant(
    conn: &mut PgConnection,
    reel_id: Uuid,
    language: &str,
) -> Result<Option<VariantRow>, DbError> {
    Ok(sqlx::query_as::<_, VariantRow>(&format!(
        "SELECT {VCOLS} FROM reel_variants WHERE reel_id = $1 AND language = $2"
    ))
    .bind(reel_id)
    .bind(language)
    .fetch_optional(conn)
    .await?)
}

pub async fn upsert_variant(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    reel_id: Uuid,
    language: &str,
    video_key: Option<&str>,
    captions_key: Option<&str>,
    audio_key: Option<&str>,
    script: Option<&str>,
    dub_status: ExtractionStatus,
) -> Result<VariantRow, DbError> {
    Ok(sqlx::query_as::<_, VariantRow>(&format!(
        "INSERT INTO reel_variants (tenant_id, reel_id, language, video_key, captions_key, audio_key, script, dub_status)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
         ON CONFLICT (reel_id, language) DO UPDATE SET
           video_key = coalesce(EXCLUDED.video_key, reel_variants.video_key),
           captions_key = coalesce(EXCLUDED.captions_key, reel_variants.captions_key),
           audio_key = coalesce(EXCLUDED.audio_key, reel_variants.audio_key),
           script = coalesce(EXCLUDED.script, reel_variants.script),
           dub_status = EXCLUDED.dub_status
         RETURNING {VCOLS}"
    ))
    .bind(tenant_id)
    .bind(reel_id)
    .bind(language)
    .bind(video_key)
    .bind(captions_key)
    .bind(audio_key)
    .bind(script)
    .bind(dub_status)
    .fetch_one(conn)
    .await?)
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct QuestionRow {
    pub id: Uuid,
    pub reel_id: Uuid,
    pub language: String,
    pub prompt: String,
    pub options: serde_json::Value,
    pub correct_key: String,
    pub explanation: Option<String>,
}

const QCOLS: &str = "id, reel_id, language, prompt, options, correct_key, explanation";

pub async fn question(conn: &mut PgConnection, id: Uuid) -> Result<Option<QuestionRow>, DbError> {
    Ok(
        sqlx::query_as::<_, QuestionRow>(&format!("SELECT {QCOLS} FROM questions WHERE id = $1"))
            .bind(id)
            .fetch_optional(conn)
            .await?,
    )
}

pub async fn questions_for_reel(
    conn: &mut PgConnection,
    reel_id: Uuid,
) -> Result<Vec<QuestionRow>, DbError> {
    Ok(sqlx::query_as::<_, QuestionRow>(&format!(
        "SELECT {QCOLS} FROM questions WHERE reel_id = $1 ORDER BY language"
    ))
    .bind(reel_id)
    .fetch_all(conn)
    .await?)
}

/// The question in `language`, falling back to the reel's master language, then any.
pub async fn question_for_reel_in(
    conn: &mut PgConnection,
    reel_id: Uuid,
    language: &str,
    master: &str,
) -> Result<Option<QuestionRow>, DbError> {
    Ok(sqlx::query_as::<_, QuestionRow>(&format!(
        "SELECT {QCOLS} FROM questions WHERE reel_id = $1 ORDER BY (language = $2) DESC, (language = $3) DESC, language LIMIT 1"
    ))
    .bind(reel_id)
    .bind(language)
    .bind(master)
    .fetch_optional(conn)
    .await?)
}

pub async fn upsert_question(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    reel_id: Uuid,
    language: &str,
    prompt: &str,
    options: &serde_json::Value,
    correct_key: &str,
    explanation: Option<&str>,
) -> Result<QuestionRow, DbError> {
    Ok(sqlx::query_as::<_, QuestionRow>(&format!(
        "INSERT INTO questions (tenant_id, reel_id, language, prompt, options, correct_key, explanation)
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         ON CONFLICT (reel_id, language) DO UPDATE SET prompt = EXCLUDED.prompt, options = EXCLUDED.options, correct_key = EXCLUDED.correct_key, explanation = EXCLUDED.explanation
         RETURNING {QCOLS}"
    ))
    .bind(tenant_id)
    .bind(reel_id)
    .bind(language)
    .bind(prompt)
    .bind(options)
    .bind(correct_key)
    .bind(explanation)
    .fetch_one(conn)
    .await?)
}

/// Recall score for a reel: correct answers over all answers (admin library, allowed on `answers`).
pub async fn recall_pct(conn: &mut PgConnection, reel_id: Uuid) -> Result<Option<i32>, DbError> {
    Ok(sqlx::query_scalar(
        "SELECT CASE WHEN count(*) > 0 THEN (100 * sum(CASE WHEN is_correct THEN 1 ELSE 0 END) / count(*))::int ELSE NULL END FROM answers WHERE reel_id = $1",
    )
    .bind(reel_id)
    .fetch_one(conn)
    .await?)
}

pub async fn plays(conn: &mut PgConnection, reel_id: Uuid) -> Result<i64, DbError> {
    Ok(
        sqlx::query_scalar("SELECT count(*) FROM watch_events WHERE reel_id = $1")
            .bind(reel_id)
            .fetch_one(conn)
            .await?,
    )
}

pub async fn count_in_topic_published(
    conn: &mut PgConnection,
    topic_id: Uuid,
) -> Result<i64, DbError> {
    Ok(sqlx::query_scalar(
        "SELECT count(*) FROM reels WHERE topic_id = $1 AND status = 'published'",
    )
    .bind(topic_id)
    .fetch_one(conn)
    .await?)
}

pub async fn count_awaiting_review(conn: &mut PgConnection) -> Result<i64, DbError> {
    Ok(
        sqlx::query_scalar("SELECT count(*) FROM reels WHERE status = 'in_review'")
            .fetch_one(conn)
            .await?,
    )
}

/// Published reels assigned to the user's groups (via topic assignments or explicit audiences) and not yet seen.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CandidateRow {
    pub reel_id: Uuid,
    pub topic_id: Uuid,
    pub language: String,
    pub published_at: DateTime<Utc>,
    pub assignment_id: Uuid,
    pub reels_per_day: i16,
    pub channel: reelearn_domain::Channel,
    pub is_mandatory: bool,
}

pub async fn unseen_candidates(
    conn: &mut PgConnection,
    user_id: Uuid,
    day: NaiveDate,
) -> Result<Vec<CandidateRow>, DbError> {
    Ok(sqlx::query_as::<_, CandidateRow>(
        "SELECT r.id AS reel_id, r.topic_id, r.language, r.published_at, a.id AS assignment_id, a.reels_per_day, a.channel, (a.is_mandatory OR t.is_mandatory) AS is_mandatory
         FROM topic_assignments a
         JOIN group_members gm ON gm.group_id = a.group_id AND gm.user_id = $1
         JOIN topics t ON t.id = a.topic_id AND t.status = 'active'
         JOIN reels r ON r.topic_id = a.topic_id AND r.status = 'published' AND r.published_at IS NOT NULL
         WHERE a.starts_on <= $2 AND (a.ends_on IS NULL OR a.ends_on >= $2)
           AND (NOT EXISTS (SELECT 1 FROM reel_audiences ra WHERE ra.reel_id = r.id)
                OR EXISTS (SELECT 1 FROM reel_audiences ra JOIN group_members gm2 ON gm2.group_id = ra.group_id AND gm2.user_id = $1 WHERE ra.reel_id = r.id AND ra.starts_on <= $2))
           AND NOT EXISTS (SELECT 1 FROM watch_events w WHERE w.user_id = $1 AND w.reel_id = r.id AND w.completed)
           AND NOT EXISTS (SELECT 1 FROM daily_plan_items i JOIN daily_plans p ON p.id = i.plan_id WHERE p.user_id = $1 AND i.reel_id = r.id AND i.reason = 'new' AND p.plan_date < $2)
         ORDER BY r.published_at, r.id",
    )
    .bind(user_id)
    .bind(day)
    .fetch_all(conn)
    .await?)
}

/// Reels a learner may browse: published to their groups or already seen.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct LibraryRow {
    pub id: Uuid,
    pub title: String,
    pub topic_name: String,
    pub presenter: serde_json::Value,
    pub duration_ms: Option<i32>,
    pub language: String,
    pub thumbnail_key: Option<String>,
    pub saved: bool,
    pub seen: bool,
    pub published_at: DateTime<Utc>,
}

pub async fn library(
    conn: &mut PgConnection,
    user_id: Uuid,
    topic_id: Option<Uuid>,
    q: Option<&str>,
    saved_only: bool,
    cursor: Option<Cursor>,
    limit: i64,
) -> Result<Vec<LibraryRow>, DbError> {
    let mut qb: QueryBuilder<sqlx::Postgres> = QueryBuilder::new(
        "SELECT r.id, r.title, t.name AS topic_name, r.presenter, r.duration_ms, r.language, r.thumbnail_key,
                EXISTS (SELECT 1 FROM saved_reels s WHERE s.user_id = ",
    );
    qb.push_bind(user_id);
    qb.push(
        " AND s.reel_id = r.id) AS saved, EXISTS (SELECT 1 FROM watch_events w WHERE w.user_id = ",
    );
    qb.push_bind(user_id);
    qb.push(
        " AND w.reel_id = r.id) AS seen, r.published_at
         FROM reels r JOIN topics t ON t.id = r.topic_id
         WHERE r.status = 'published' AND r.published_at IS NOT NULL
           AND (EXISTS (SELECT 1 FROM topic_assignments a JOIN group_members gm ON gm.group_id = a.group_id WHERE gm.user_id = ",
    );
    qb.push_bind(user_id);
    qb.push(
        " AND a.topic_id = r.topic_id) OR EXISTS (SELECT 1 FROM watch_events w WHERE w.user_id = ",
    );
    qb.push_bind(user_id);
    qb.push(" AND w.reel_id = r.id))");
    if let Some(t) = topic_id {
        qb.push(" AND r.topic_id = ");
        qb.push_bind(t);
    }
    if let Some(q) = q.filter(|s| !s.trim().is_empty()) {
        qb.push(" AND (r.title ILIKE ");
        qb.push_bind(format!("%{}%", q.trim()));
        qb.push(" OR t.name ILIKE ");
        qb.push_bind(format!("%{}%", q.trim()));
        qb.push(" OR coalesce(t.name_ar,'') ILIKE ");
        qb.push_bind(format!("%{}%", q.trim()));
        qb.push(")");
    }
    if saved_only {
        qb.push(" AND EXISTS (SELECT 1 FROM saved_reels s WHERE s.user_id = ");
        qb.push_bind(user_id);
        qb.push(" AND s.reel_id = r.id)");
    }
    push_cursor_filter(&mut qb, "r.published_at", "r.id", cursor);
    qb.push(" ORDER BY r.published_at DESC, r.id DESC LIMIT ");
    qb.push_bind(limit + 1);
    Ok(qb.build_query_as::<LibraryRow>().fetch_all(conn).await?)
}

/// May this learner play the reel? Published to their groups, or in any of their plans.
pub async fn learner_may_play(
    conn: &mut PgConnection,
    user_id: Uuid,
    reel_id: Uuid,
) -> Result<bool, DbError> {
    Ok(sqlx::query_scalar(
        "SELECT EXISTS (
           SELECT 1 FROM reels r WHERE r.id = $2 AND r.status = 'published' AND (
             EXISTS (SELECT 1 FROM topic_assignments a JOIN group_members gm ON gm.group_id = a.group_id WHERE gm.user_id = $1 AND a.topic_id = r.topic_id)
             OR EXISTS (SELECT 1 FROM daily_plan_items i JOIN daily_plans p ON p.id = i.plan_id WHERE p.user_id = $1 AND i.reel_id = r.id)
           ))",
    )
    .bind(user_id)
    .bind(reel_id)
    .fetch_one(conn)
    .await?)
}

/// Groups that have an assignment for the reel's topic and contain the user (for "why am I seeing this").
pub async fn assigning_group_name(
    conn: &mut PgConnection,
    user_id: Uuid,
    reel_id: Uuid,
) -> Result<Option<String>, DbError> {
    Ok(sqlx::query_scalar(
        "SELECT g.name FROM reels r JOIN topic_assignments a ON a.topic_id = r.topic_id JOIN group_members gm ON gm.group_id = a.group_id AND gm.user_id = $1
         JOIN groups g ON g.id = a.group_id WHERE r.id = $2 ORDER BY a.is_mandatory DESC, g.name LIMIT 1",
    )
    .bind(user_id)
    .bind(reel_id)
    .fetch_optional(conn)
    .await?)
}

/// Seed helper: make a published reel look older so plan ordering is deterministic.
pub async fn backdate_published(
    conn: &mut PgConnection,
    id: Uuid,
    days_ago: i32,
) -> Result<(), DbError> {
    sqlx::query("UPDATE reels SET published_at = now() - make_interval(days => $2), updated_at = now() - make_interval(days => $2) WHERE id = $1")
        .bind(id)
        .bind(days_ago)
        .execute(conn)
        .await?;
    Ok(())
}
