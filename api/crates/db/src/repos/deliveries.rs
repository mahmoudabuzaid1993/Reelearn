//! Scheduled sends and the WhatsApp message log.

use crate::error::DbError;
use chrono::{DateTime, NaiveDate, Utc};
use reelearn_domain::{Channel, DeliveryStatus};
use sqlx::PgConnection;
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct DeliveryRow {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub user_id: Uuid,
    pub plan_id: Option<Uuid>,
    pub channel: Channel,
    pub kind: String,
    pub status: DeliveryStatus,
    pub scheduled_for: DateTime<Utc>,
    pub sent_at: Option<DateTime<Utc>>,
    pub provider_ref: Option<String>,
    pub error: Option<String>,
}

const COLS: &str = "id, tenant_id, user_id, plan_id, channel, kind, status, scheduled_for, sent_at, provider_ref, error";

pub async fn schedule(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    user_id: Uuid,
    plan_id: Option<Uuid>,
    channel: Channel,
    kind: &str,
    scheduled_for: DateTime<Utc>,
) -> Result<DeliveryRow, DbError> {
    Ok(sqlx::query_as::<_, DeliveryRow>(&format!(
        "INSERT INTO deliveries (tenant_id, user_id, plan_id, channel, kind, scheduled_for) VALUES ($1, $2, $3, $4, $5, $6) RETURNING {COLS}"
    ))
    .bind(tenant_id)
    .bind(user_id)
    .bind(plan_id)
    .bind(channel)
    .bind(kind)
    .bind(scheduled_for)
    .fetch_one(conn)
    .await?)
}

pub async fn get(conn: &mut PgConnection, id: Uuid) -> Result<Option<DeliveryRow>, DbError> {
    Ok(sqlx::query_as::<_, DeliveryRow>(&format!(
        "SELECT {COLS} FROM deliveries WHERE id = $1 FOR UPDATE"
    ))
    .bind(id)
    .fetch_optional(conn)
    .await?)
}

/// Cross-tenant: due deliveries for the scheduler fan-out (uses the partial `deliveries_due_idx`).
pub async fn due_admin(conn: &mut PgConnection, limit: i64) -> Result<Vec<DeliveryRow>, DbError> {
    Ok(sqlx::query_as::<_, DeliveryRow>(&format!("SELECT {COLS} FROM deliveries WHERE status = 'scheduled' AND scheduled_for <= now() ORDER BY scheduled_for LIMIT $1"))
        .bind(limit)
        .fetch_all(conn)
        .await?)
}

pub async fn mark(
    conn: &mut PgConnection,
    id: Uuid,
    status: DeliveryStatus,
    provider_ref: Option<&str>,
    error: Option<&str>,
) -> Result<(), DbError> {
    sqlx::query("UPDATE deliveries SET status = $2, provider_ref = coalesce($3, provider_ref), error = $4, sent_at = CASE WHEN $2 = 'sent'::delivery_status_t THEN now() ELSE sent_at END WHERE id = $1")
        .bind(id)
        .bind(status)
        .bind(provider_ref)
        .bind(error)
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn mark_by_provider_ref(
    conn: &mut PgConnection,
    provider_ref: &str,
    status: DeliveryStatus,
) -> Result<u64, DbError> {
    Ok(sqlx::query("UPDATE deliveries SET status = $2 WHERE provider_ref = $1 AND status IN ('sent','delivered')")
        .bind(provider_ref)
        .bind(status)
        .execute(conn)
        .await?
        .rows_affected())
}

/// Was a nudge already scheduled for this user today (tenant-local day boundaries approximated by UTC day)?
pub async fn nudged_on(
    conn: &mut PgConnection,
    user_ids: &[Uuid],
    day: NaiveDate,
) -> Result<Vec<Uuid>, DbError> {
    Ok(sqlx::query_scalar(
        "SELECT DISTINCT user_id FROM deliveries WHERE user_id = ANY($1) AND kind = 'nudge' AND scheduled_for::date = $2 AND status <> 'cancelled'",
    )
    .bind(user_ids)
    .bind(day)
    .fetch_all(conn)
    .await?)
}

pub async fn exists_for_plan(
    conn: &mut PgConnection,
    plan_id: Uuid,
    kind: &str,
) -> Result<bool, DbError> {
    Ok(sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM deliveries WHERE plan_id = $1 AND kind = $2)",
    )
    .bind(plan_id)
    .bind(kind)
    .fetch_one(conn)
    .await?)
}

pub async fn log_whatsapp(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    user_id: Option<Uuid>,
    phone: &str,
    direction: &str,
    body: Option<&str>,
    payload: Option<serde_json::Value>,
    provider_ref: Option<&str>,
) -> Result<(), DbError> {
    sqlx::query("INSERT INTO whatsapp_messages (tenant_id, user_id, phone_e164, direction, body, payload, provider_ref) VALUES ($1, $2, $3, $4, $5, $6, $7)")
        .bind(tenant_id)
        .bind(user_id)
        .bind(phone)
        .bind(direction)
        .bind(body)
        .bind(payload)
        .bind(provider_ref)
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn whatsapp_inbound_seen(
    conn: &mut PgConnection,
    provider_ref: &str,
) -> Result<bool, DbError> {
    Ok(sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM whatsapp_messages WHERE provider_ref = $1 AND direction = 'in')").bind(provider_ref).fetch_one(conn).await?)
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct WhatsappMessageRow {
    pub id: Uuid,
    pub direction: String,
    pub body: Option<String>,
    pub payload: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

pub async fn recent_whatsapp(
    conn: &mut PgConnection,
    phone: &str,
    limit: i64,
) -> Result<Vec<WhatsappMessageRow>, DbError> {
    Ok(sqlx::query_as::<_, WhatsappMessageRow>("SELECT id, direction, body, payload, created_at FROM whatsapp_messages WHERE phone_e164 = $1 ORDER BY created_at DESC LIMIT $2")
        .bind(phone)
        .bind(limit)
        .fetch_all(conn)
        .await?)
}
