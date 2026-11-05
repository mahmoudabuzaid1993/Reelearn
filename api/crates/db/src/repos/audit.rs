use crate::error::DbError;
use crate::repos::push_cursor_filter;
use chrono::{DateTime, Utc};
use reelearn_domain::cursor::Cursor;
use sqlx::{PgConnection, QueryBuilder};
use uuid::Uuid;

pub async fn record(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    actor_id: Option<Uuid>,
    action: &str,
    entity: &str,
    entity_id: Option<Uuid>,
    diff: Option<serde_json::Value>,
    ip: Option<&str>,
) -> Result<(), DbError> {
    sqlx::query("INSERT INTO audit_log (tenant_id, actor_id, action, entity, entity_id, diff, ip) VALUES ($1, $2, $3, $4, $5, $6, $7::inet)")
        .bind(tenant_id)
        .bind(actor_id)
        .bind(action)
        .bind(entity)
        .bind(entity_id)
        .bind(diff)
        .bind(ip)
        .execute(conn)
        .await?;
    Ok(())
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AuditRow {
    pub id: Uuid,
    pub actor_id: Option<Uuid>,
    pub actor_name: Option<String>,
    pub action: String,
    pub entity: String,
    pub entity_id: Option<Uuid>,
    pub diff: Option<serde_json::Value>,
    pub at: DateTime<Utc>,
}

pub async fn list(
    conn: &mut PgConnection,
    entity: Option<&str>,
    entity_id: Option<Uuid>,
    cursor: Option<Cursor>,
    limit: i64,
) -> Result<Vec<AuditRow>, DbError> {
    let mut qb: QueryBuilder<sqlx::Postgres> = QueryBuilder::new(
        "SELECT a.id, a.actor_id, u.full_name AS actor_name, a.action, a.entity, a.entity_id, a.diff, a.at FROM audit_log a LEFT JOIN users u ON u.id = a.actor_id WHERE true",
    );
    if let Some(e) = entity {
        qb.push(" AND a.entity = ");
        qb.push_bind(e.to_string());
    }
    if let Some(id) = entity_id {
        qb.push(" AND a.entity_id = ");
        qb.push_bind(id);
    }
    push_cursor_filter(&mut qb, "a.at", "a.id", cursor);
    qb.push(" ORDER BY a.at DESC, a.id DESC LIMIT ");
    qb.push_bind(limit + 1);
    Ok(qb.build_query_as::<AuditRow>().fetch_all(conn).await?)
}
