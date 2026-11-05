use crate::error::DbError;
use chrono::{DateTime, Utc};
use reelearn_domain::Region;
use sqlx::PgConnection;
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TenantRow {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub region: Region,
    pub default_language: String,
    pub languages: Vec<String>,
    pub timezone: String,
    pub logo_key: Option<String>,
    pub settings: serde_json::Value,
    pub credits: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

const COLS: &str = "id, slug, name, region, default_language, languages, timezone, logo_key, settings, credits, created_at, updated_at";

pub async fn find_by_slug(
    conn: &mut PgConnection,
    slug: &str,
) -> Result<Option<TenantRow>, DbError> {
    Ok(
        sqlx::query_as::<_, TenantRow>(&format!("SELECT {COLS} FROM tenants WHERE slug = $1"))
            .bind(slug)
            .fetch_optional(conn)
            .await?,
    )
}

pub async fn get(conn: &mut PgConnection, id: Uuid) -> Result<TenantRow, DbError> {
    sqlx::query_as::<_, TenantRow>(&format!("SELECT {COLS} FROM tenants WHERE id = $1"))
        .bind(id)
        .fetch_optional(conn)
        .await?
        .ok_or(DbError::NotFound("tenant"))
}

pub async fn list_all(conn: &mut PgConnection) -> Result<Vec<TenantRow>, DbError> {
    Ok(
        sqlx::query_as::<_, TenantRow>(&format!("SELECT {COLS} FROM tenants ORDER BY created_at"))
            .fetch_all(conn)
            .await?,
    )
}

pub struct NewTenant<'a> {
    pub slug: &'a str,
    pub name: &'a str,
    pub region: Region,
    pub default_language: &'a str,
    pub languages: &'a [String],
    pub timezone: &'a str,
    pub settings: serde_json::Value,
    pub credits: i32,
}

pub async fn create(conn: &mut PgConnection, t: NewTenant<'_>) -> Result<TenantRow, DbError> {
    Ok(sqlx::query_as::<_, TenantRow>(&format!(
        "INSERT INTO tenants (slug, name, region, default_language, languages, timezone, settings, credits)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING {COLS}"
    ))
    .bind(t.slug)
    .bind(t.name)
    .bind(t.region)
    .bind(t.default_language)
    .bind(t.languages)
    .bind(t.timezone)
    .bind(t.settings)
    .bind(t.credits)
    .fetch_one(conn)
    .await?)
}

#[derive(Debug, Default)]
pub struct TenantPatch {
    pub name: Option<String>,
    pub languages: Option<Vec<String>>,
    pub default_language: Option<String>,
    pub timezone: Option<String>,
    pub settings: Option<serde_json::Value>,
    pub logo_key: Option<String>,
}

pub async fn update(
    conn: &mut PgConnection,
    id: Uuid,
    p: TenantPatch,
) -> Result<TenantRow, DbError> {
    Ok(sqlx::query_as::<_, TenantRow>(&format!(
        "UPDATE tenants SET
           name = coalesce($2, name),
           languages = coalesce($3, languages),
           default_language = coalesce($4, default_language),
           timezone = coalesce($5, timezone),
           settings = coalesce($6, settings),
           logo_key = coalesce($7, logo_key)
         WHERE id = $1 RETURNING {COLS}"
    ))
    .bind(id)
    .bind(p.name)
    .bind(p.languages)
    .bind(p.default_language)
    .bind(p.timezone)
    .bind(p.settings)
    .bind(p.logo_key)
    .fetch_one(conn)
    .await?)
}

/// Atomically spend credits; fails without touching the row when the balance is short.
pub async fn spend_credits(conn: &mut PgConnection, id: Uuid, amount: i32) -> Result<i32, DbError> {
    if amount <= 0 {
        return Ok(get(&mut *conn, id).await?.credits);
    }
    let updated: Option<i32> = sqlx::query_scalar("UPDATE tenants SET credits = credits - $2 WHERE id = $1 AND credits >= $2 RETURNING credits")
        .bind(id)
        .bind(amount)
        .fetch_optional(&mut *conn)
        .await?;
    match updated {
        Some(c) => Ok(c),
        None => {
            let available = get(conn, id).await?.credits;
            Err(DbError::InsufficientCredits {
                needed: amount,
                available,
            })
        }
    }
}

pub async fn refund_credits(conn: &mut PgConnection, id: Uuid, amount: i32) -> Result<(), DbError> {
    sqlx::query("UPDATE tenants SET credits = credits + $2 WHERE id = $1")
        .bind(id)
        .bind(amount.max(0))
        .execute(conn)
        .await?;
    Ok(())
}
