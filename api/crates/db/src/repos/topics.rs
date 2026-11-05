use crate::error::DbError;
use chrono::{DateTime, NaiveDate, Utc};
use reelearn_domain::Channel;
use sqlx::PgConnection;
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TopicRow {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub name_ar: Option<String>,
    pub description: Option<String>,
    pub compliance_tag: Option<String>,
    pub is_mandatory: bool,
    pub status: String,
    pub published_count: i64,
    pub in_review_count: i64,
    pub draft_count: i64,
    pub recall_pct: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

const SQL_TOPICS: &str = "SELECT t.id, t.slug, t.name, t.name_ar, t.description, t.compliance_tag, t.is_mandatory, t.status,
    (SELECT count(*) FROM reels r WHERE r.topic_id = t.id AND r.status = 'published') AS published_count,
    (SELECT count(*) FROM reels r WHERE r.topic_id = t.id AND r.status = 'in_review') AS in_review_count,
    (SELECT count(*) FROM reels r WHERE r.topic_id = t.id AND r.status IN ('draft','approved')) AS draft_count,
    (SELECT CASE WHEN sum(s.answers_total) > 0 THEN (100 * sum(s.answers_correct) / sum(s.answers_total))::int ELSE NULL END
       FROM daily_topic_stats s WHERE s.topic_id = t.id) AS recall_pct,
    t.created_at, t.updated_at
  FROM topics t";

pub async fn list(conn: &mut PgConnection) -> Result<Vec<TopicRow>, DbError> {
    Ok(sqlx::query_as::<_, TopicRow>(&format!(
        "{SQL_TOPICS} ORDER BY t.is_mandatory DESC, t.name"
    ))
    .fetch_all(conn)
    .await?)
}

pub async fn get(conn: &mut PgConnection, id: Uuid) -> Result<TopicRow, DbError> {
    sqlx::query_as::<_, TopicRow>(&format!("{SQL_TOPICS} WHERE t.id = $1"))
        .bind(id)
        .fetch_optional(conn)
        .await?
        .ok_or(DbError::NotFound("topic"))
}

pub async fn find_by_slug(
    conn: &mut PgConnection,
    slug: &str,
) -> Result<Option<TopicRow>, DbError> {
    Ok(
        sqlx::query_as::<_, TopicRow>(&format!("{SQL_TOPICS} WHERE t.slug = $1"))
            .bind(slug)
            .fetch_optional(conn)
            .await?,
    )
}

pub struct TopicWrite<'a> {
    pub slug: &'a str,
    pub name: &'a str,
    pub name_ar: Option<&'a str>,
    pub description: Option<&'a str>,
    pub compliance_tag: Option<&'a str>,
    pub is_mandatory: bool,
}

pub async fn create(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    t: TopicWrite<'_>,
) -> Result<TopicRow, DbError> {
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO topics (tenant_id, slug, name, name_ar, description, compliance_tag, is_mandatory) VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING id",
    )
    .bind(tenant_id)
    .bind(t.slug)
    .bind(t.name)
    .bind(t.name_ar)
    .bind(t.description)
    .bind(t.compliance_tag)
    .bind(t.is_mandatory)
    .fetch_one(&mut *conn)
    .await?;
    get(conn, id).await
}

#[derive(Debug, Default)]
pub struct TopicPatch {
    pub slug: Option<String>,
    pub name: Option<String>,
    pub name_ar: Option<String>,
    pub description: Option<String>,
    pub compliance_tag: Option<String>,
    pub is_mandatory: Option<bool>,
    pub status: Option<String>,
}

pub async fn update(conn: &mut PgConnection, id: Uuid, p: TopicPatch) -> Result<TopicRow, DbError> {
    sqlx::query(
        "UPDATE topics SET slug = coalesce($2, slug), name = coalesce($3, name), name_ar = coalesce($4, name_ar), description = coalesce($5, description),
         compliance_tag = coalesce($6, compliance_tag), is_mandatory = coalesce($7, is_mandatory), status = coalesce($8, status) WHERE id = $1",
    )
    .bind(id)
    .bind(p.slug)
    .bind(p.name)
    .bind(p.name_ar)
    .bind(p.description)
    .bind(p.compliance_tag)
    .bind(p.is_mandatory)
    .bind(p.status)
    .execute(&mut *conn)
    .await?;
    get(conn, id).await
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AssignmentRow {
    pub id: Uuid,
    pub topic_id: Uuid,
    pub group_id: Uuid,
    pub group_name: String,
    pub reels_per_day: i16,
    pub channel: Channel,
    pub is_mandatory: bool,
    pub starts_on: NaiveDate,
    pub ends_on: Option<NaiveDate>,
}

const SQL_ASSIGNMENTS: &str = "SELECT a.id, a.topic_id, a.group_id, g.name AS group_name, a.reels_per_day, a.channel, a.is_mandatory, a.starts_on, a.ends_on
  FROM topic_assignments a JOIN groups g ON g.id = a.group_id";

pub async fn assignments_for_topic(
    conn: &mut PgConnection,
    topic_id: Uuid,
) -> Result<Vec<AssignmentRow>, DbError> {
    Ok(sqlx::query_as::<_, AssignmentRow>(&format!(
        "{SQL_ASSIGNMENTS} WHERE a.topic_id = $1 ORDER BY g.name"
    ))
    .bind(topic_id)
    .fetch_all(conn)
    .await?)
}

/// Active assignments reaching a user through any of their groups on `day`.
pub async fn assignments_for_user(
    conn: &mut PgConnection,
    user_id: Uuid,
    day: NaiveDate,
) -> Result<Vec<AssignmentRow>, DbError> {
    Ok(sqlx::query_as::<_, AssignmentRow>(&format!(
        "{SQL_ASSIGNMENTS} JOIN group_members gm ON gm.group_id = a.group_id JOIN topics t ON t.id = a.topic_id
         WHERE gm.user_id = $1 AND a.starts_on <= $2 AND (a.ends_on IS NULL OR a.ends_on >= $2) AND t.status = 'active'
         ORDER BY a.is_mandatory DESC, a.created_at"
    ))
    .bind(user_id)
    .bind(day)
    .fetch_all(conn)
    .await?)
}

pub struct AssignmentWrite {
    pub group_id: Uuid,
    pub reels_per_day: i16,
    pub channel: Channel,
    pub is_mandatory: bool,
    pub starts_on: Option<NaiveDate>,
    pub ends_on: Option<NaiveDate>,
}

pub async fn replace_assignments(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    topic_id: Uuid,
    rows: &[AssignmentWrite],
) -> Result<Vec<AssignmentRow>, DbError> {
    sqlx::query("DELETE FROM topic_assignments WHERE topic_id = $1")
        .bind(topic_id)
        .execute(&mut *conn)
        .await?;
    for r in rows {
        sqlx::query(
            "INSERT INTO topic_assignments (tenant_id, topic_id, group_id, reels_per_day, channel, is_mandatory, starts_on, ends_on)
             VALUES ($1, $2, $3, $4, $5, $6, coalesce($7, CURRENT_DATE), $8)",
        )
        .bind(tenant_id)
        .bind(topic_id)
        .bind(r.group_id)
        .bind(r.reels_per_day)
        .bind(r.channel)
        .bind(r.is_mandatory)
        .bind(r.starts_on)
        .bind(r.ends_on)
        .execute(&mut *conn)
        .await?;
    }
    assignments_for_topic(conn, topic_id).await
}

pub async fn upsert_assignment(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    topic_id: Uuid,
    r: &AssignmentWrite,
) -> Result<(), DbError> {
    sqlx::query(
        "INSERT INTO topic_assignments (tenant_id, topic_id, group_id, reels_per_day, channel, is_mandatory, starts_on, ends_on)
         VALUES ($1, $2, $3, $4, $5, $6, coalesce($7, CURRENT_DATE), $8)
         ON CONFLICT (topic_id, group_id) DO UPDATE SET reels_per_day = EXCLUDED.reels_per_day, channel = EXCLUDED.channel, is_mandatory = EXCLUDED.is_mandatory, starts_on = EXCLUDED.starts_on, ends_on = EXCLUDED.ends_on",
    )
    .bind(tenant_id)
    .bind(topic_id)
    .bind(r.group_id)
    .bind(r.reels_per_day)
    .bind(r.channel)
    .bind(r.is_mandatory)
    .bind(r.starts_on)
    .bind(r.ends_on)
    .execute(conn)
    .await?;
    Ok(())
}

/// Topic ids assigned to any of the user's groups (for readiness and library views).
pub async fn topic_ids_for_user(
    conn: &mut PgConnection,
    user_id: Uuid,
) -> Result<Vec<Uuid>, DbError> {
    Ok(sqlx::query_scalar(
        "SELECT DISTINCT a.topic_id FROM topic_assignments a JOIN group_members gm ON gm.group_id = a.group_id WHERE gm.user_id = $1",
    )
    .bind(user_id)
    .fetch_all(conn)
    .await?)
}

/// The first of the user's groups that has an assignment for the topic (for daily_topic_stats attribution).
pub async fn group_for_user_topic(
    conn: &mut PgConnection,
    user_id: Uuid,
    topic_id: Uuid,
) -> Result<Option<Uuid>, DbError> {
    Ok(sqlx::query_scalar(
        "SELECT a.group_id FROM topic_assignments a JOIN group_members gm ON gm.group_id = a.group_id
         WHERE gm.user_id = $1 AND a.topic_id = $2 ORDER BY a.created_at LIMIT 1",
    )
    .bind(user_id)
    .bind(topic_id)
    .fetch_optional(conn)
    .await?)
}
