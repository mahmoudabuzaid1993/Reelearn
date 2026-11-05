use crate::error::DbError;
use chrono::{DateTime, NaiveDate, Utc};
use reelearn_domain::{PlanItemKind, PlanItemReason, PlanItemState};
use sqlx::PgConnection;
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PlanRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub plan_date: NaiveDate,
    pub status: String,
    pub item_count: i16,
    pub done_count: i16,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PlanItemRow {
    pub id: Uuid,
    pub plan_id: Uuid,
    pub position: i16,
    pub kind: PlanItemKind,
    pub reason: PlanItemReason,
    pub reel_id: Option<Uuid>,
    pub question_id: Option<Uuid>,
    pub state: PlanItemState,
}

const PCOLS: &str =
    "id, user_id, plan_date, status, item_count, done_count, completed_at, created_at";
const ICOLS: &str = "id, plan_id, position, kind, reason, reel_id, question_id, state";

pub async fn find_for_date(
    conn: &mut PgConnection,
    user_id: Uuid,
    day: NaiveDate,
) -> Result<Option<PlanRow>, DbError> {
    Ok(sqlx::query_as::<_, PlanRow>(&format!(
        "SELECT {PCOLS} FROM daily_plans WHERE user_id = $1 AND plan_date = $2"
    ))
    .bind(user_id)
    .bind(day)
    .fetch_optional(conn)
    .await?)
}

pub async fn get(conn: &mut PgConnection, id: Uuid) -> Result<PlanRow, DbError> {
    sqlx::query_as::<_, PlanRow>(&format!("SELECT {PCOLS} FROM daily_plans WHERE id = $1"))
        .bind(id)
        .fetch_optional(conn)
        .await?
        .ok_or(DbError::NotFound("plan"))
}

pub async fn create(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    user_id: Uuid,
    day: NaiveDate,
    item_count: i16,
) -> Result<PlanRow, DbError> {
    Ok(sqlx::query_as::<_, PlanRow>(&format!(
        "INSERT INTO daily_plans (tenant_id, user_id, plan_date, status, item_count) VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (tenant_id, user_id, plan_date) DO UPDATE SET plan_date = EXCLUDED.plan_date RETURNING {PCOLS}"
    ))
    .bind(tenant_id)
    .bind(user_id)
    .bind(day)
    .bind(if item_count == 0 { "done" } else { "open" })
    .bind(item_count)
    .fetch_one(conn)
    .await?)
}

pub async fn insert_item(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    plan_id: Uuid,
    position: i16,
    kind: PlanItemKind,
    reason: PlanItemReason,
    reel_id: Option<Uuid>,
    question_id: Option<Uuid>,
) -> Result<PlanItemRow, DbError> {
    Ok(sqlx::query_as::<_, PlanItemRow>(&format!(
        "INSERT INTO daily_plan_items (tenant_id, plan_id, position, kind, reason, reel_id, question_id) VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING {ICOLS}"
    ))
    .bind(tenant_id)
    .bind(plan_id)
    .bind(position)
    .bind(kind)
    .bind(reason)
    .bind(reel_id)
    .bind(question_id)
    .fetch_one(conn)
    .await?)
}

pub async fn items(conn: &mut PgConnection, plan_id: Uuid) -> Result<Vec<PlanItemRow>, DbError> {
    Ok(sqlx::query_as::<_, PlanItemRow>(&format!(
        "SELECT {ICOLS} FROM daily_plan_items WHERE plan_id = $1 ORDER BY position"
    ))
    .bind(plan_id)
    .fetch_all(conn)
    .await?)
}

pub async fn item(conn: &mut PgConnection, id: Uuid) -> Result<Option<PlanItemRow>, DbError> {
    Ok(sqlx::query_as::<_, PlanItemRow>(&format!(
        "SELECT {ICOLS} FROM daily_plan_items WHERE id = $1 FOR UPDATE"
    ))
    .bind(id)
    .fetch_optional(conn)
    .await?)
}

/// Today's plan item for a question, when the client did not send `plan_item_id`.
pub async fn item_for_question(
    conn: &mut PgConnection,
    plan_id: Uuid,
    question_id: Uuid,
) -> Result<Option<PlanItemRow>, DbError> {
    Ok(sqlx::query_as::<_, PlanItemRow>(&format!("SELECT {ICOLS} FROM daily_plan_items WHERE plan_id = $1 AND question_id = $2 ORDER BY position LIMIT 1 FOR UPDATE"))
        .bind(plan_id)
        .bind(question_id)
        .fetch_optional(conn)
        .await?)
}

pub async fn item_for_reel(
    conn: &mut PgConnection,
    plan_id: Uuid,
    reel_id: Uuid,
) -> Result<Option<PlanItemRow>, DbError> {
    Ok(sqlx::query_as::<_, PlanItemRow>(&format!("SELECT {ICOLS} FROM daily_plan_items WHERE plan_id = $1 AND reel_id = $2 AND kind = 'reel' ORDER BY position LIMIT 1 FOR UPDATE"))
        .bind(plan_id)
        .bind(reel_id)
        .fetch_optional(conn)
        .await?)
}

pub async fn set_item_state(
    conn: &mut PgConnection,
    id: Uuid,
    state: PlanItemState,
) -> Result<(), DbError> {
    sqlx::query("UPDATE daily_plan_items SET state = $2 WHERE id = $1")
        .bind(id)
        .bind(state)
        .execute(conn)
        .await?;
    Ok(())
}

/// Recount done items; mark the plan done when every item is answered. Returns the plan.
pub async fn refresh_progress(conn: &mut PgConnection, plan_id: Uuid) -> Result<PlanRow, DbError> {
    Ok(sqlx::query_as::<_, PlanRow>(&format!(
        "UPDATE daily_plans p SET
           done_count = (SELECT count(*) FROM daily_plan_items i WHERE i.plan_id = p.id AND i.state = 'answered'),
           status = CASE WHEN (SELECT count(*) FROM daily_plan_items i WHERE i.plan_id = p.id AND i.state <> 'answered') = 0 THEN 'done' ELSE 'open' END,
           completed_at = CASE WHEN (SELECT count(*) FROM daily_plan_items i WHERE i.plan_id = p.id AND i.state <> 'answered') = 0 THEN coalesce(p.completed_at, now()) ELSE NULL END
         WHERE p.id = $1 RETURNING {PCOLS}"
    ))
    .bind(plan_id)
    .fetch_one(conn)
    .await?)
}

/// Remove pending items for an archived reel from today's and future plans.
pub async fn drop_pending_items_for_reel(
    conn: &mut PgConnection,
    reel_id: Uuid,
    from: NaiveDate,
) -> Result<u64, DbError> {
    let affected = sqlx::query(
        "DELETE FROM daily_plan_items i USING daily_plans p WHERE p.id = i.plan_id AND i.reel_id = $1 AND i.state = 'pending' AND p.plan_date >= $2",
    )
    .bind(reel_id)
    .bind(from)
    .execute(&mut *conn)
    .await?
    .rows_affected();
    sqlx::query(
        "UPDATE daily_plans p SET item_count = (SELECT count(*) FROM daily_plan_items i WHERE i.plan_id = p.id) WHERE p.plan_date >= $1",
    )
    .bind(from)
    .execute(conn)
    .await?;
    Ok(affected)
}
