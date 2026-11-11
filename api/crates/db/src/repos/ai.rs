//! AI providers: platform keys and plans (admin connection), company usage metering, Jev's
//! draft reviews and job-title relevance (tenant connection).

use crate::error::DbError;
use chrono::{DateTime, Utc};
use reelearn_domain::ai::AiPlan;
use sqlx::PgConnection;
use uuid::Uuid;

// ---------------------------------------------------------------- platform (admin connection)

pub async fn is_platform_operator_admin(
    conn: &mut PgConnection,
    user_id: Uuid,
) -> Result<bool, DbError> {
    Ok(sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (SELECT 1 FROM platform_operators WHERE user_id = $1)",
    )
    .bind(user_id)
    .fetch_one(conn)
    .await?)
}

pub async fn grant_platform_operator_admin(
    conn: &mut PgConnection,
    user_id: Uuid,
) -> Result<(), DbError> {
    sqlx::query("INSERT INTO platform_operators (user_id) VALUES ($1) ON CONFLICT DO NOTHING")
        .bind(user_id)
        .execute(conn)
        .await?;
    Ok(())
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PlatformIntegrationRow {
    pub kind: String,
    pub status: String,
    pub config_enc: Option<Vec<u8>>,
    pub last_error: Option<String>,
    pub updated_at: DateTime<Utc>,
}

pub async fn platform_integrations_admin(
    conn: &mut PgConnection,
) -> Result<Vec<PlatformIntegrationRow>, DbError> {
    Ok(sqlx::query_as::<_, PlatformIntegrationRow>(
        "SELECT kind, status, config_enc, last_error, updated_at FROM platform_integrations ORDER BY kind",
    )
    .fetch_all(conn)
    .await?)
}

pub async fn upsert_platform_integration_admin(
    conn: &mut PgConnection,
    kind: &str,
    status: &str,
    config_enc: &[u8],
    by: Uuid,
) -> Result<PlatformIntegrationRow, DbError> {
    Ok(sqlx::query_as::<_, PlatformIntegrationRow>(
        "INSERT INTO platform_integrations (kind, status, config_enc, updated_by, last_error)
         VALUES ($1, $2, $3, $4, NULL)
         ON CONFLICT (kind) DO UPDATE SET status = EXCLUDED.status, config_enc = EXCLUDED.config_enc,
           updated_by = EXCLUDED.updated_by, last_error = NULL, updated_at = now()
         RETURNING kind, status, config_enc, last_error, updated_at",
    )
    .bind(kind)
    .bind(status)
    .bind(config_enc)
    .bind(by)
    .fetch_one(conn)
    .await?)
}

pub async fn set_platform_status_admin(
    conn: &mut PgConnection,
    kind: &str,
    status: &str,
    error: Option<&str>,
) -> Result<(), DbError> {
    sqlx::query(
        "UPDATE platform_integrations SET status = $2, last_error = $3, updated_at = now() WHERE kind = $1",
    )
    .bind(kind)
    .bind(status)
    .bind(error)
    .execute(conn)
    .await?;
    Ok(())
}

/// The company's AI plan; works on either connection (tenants is readable by the app role).
pub async fn plan(conn: &mut PgConnection, tenant_id: Uuid) -> Result<AiPlan, DbError> {
    let v: Option<serde_json::Value> =
        sqlx::query_scalar("SELECT ai_plan FROM tenants WHERE id = $1")
            .bind(tenant_id)
            .fetch_optional(conn)
            .await?;
    Ok(v.and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default())
}

pub async fn set_plan_admin(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    plan: &AiPlan,
) -> Result<(), DbError> {
    let rows = sqlx::query("UPDATE tenants SET ai_plan = $2, updated_at = now() WHERE id = $1")
        .bind(tenant_id)
        .bind(serde_json::to_value(plan).unwrap_or_default())
        .execute(conn)
        .await?
        .rows_affected();
    if rows == 0 {
        return Err(DbError::NotFound("tenant"));
    }
    Ok(())
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TenantAiRow {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub ai_plan: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub calls: i64,
    pub company_calls: i64,
    pub platform_calls: i64,
    pub platform_cost: f64,
    pub company_cost: f64,
    pub unpriced_calls: i64,
}

/// Every company with this period's AI usage split by whose key paid.
pub async fn tenants_with_usage_admin(
    conn: &mut PgConnection,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> Result<Vec<TenantAiRow>, DbError> {
    Ok(sqlx::query_as::<_, TenantAiRow>(
        "SELECT t.id, t.slug, t.name, t.ai_plan, t.created_at,
                count(u.id)::bigint AS calls,
                count(u.id) FILTER (WHERE u.key_source = 'company')::bigint AS company_calls,
                count(u.id) FILTER (WHERE u.key_source = 'platform')::bigint AS platform_calls,
                coalesce(sum(u.cost_usd) FILTER (WHERE u.key_source = 'platform'), 0)::float8 AS platform_cost,
                coalesce(sum(u.cost_usd) FILTER (WHERE u.key_source = 'company'), 0)::float8 AS company_cost,
                count(u.id) FILTER (WHERE u.cost_usd IS NULL)::bigint AS unpriced_calls
         FROM tenants t
         LEFT JOIN ai_usage u ON u.tenant_id = t.id AND u.at >= $1 AND u.at < $2
         GROUP BY t.id ORDER BY platform_cost DESC, t.name",
    )
    .bind(from)
    .bind(to)
    .fetch_all(conn)
    .await?)
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct UsageLineRow {
    pub tenant_slug: String,
    pub provider: String,
    pub capability: String,
    pub key_source: String,
    pub model: Option<String>,
    pub calls: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub units: i64,
    pub cost_usd: Option<f64>,
    pub unpriced_calls: i64,
}

/// Billing lines across every company for a period (the invoice export).
pub async fn usage_lines_admin(
    conn: &mut PgConnection,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> Result<Vec<UsageLineRow>, DbError> {
    Ok(sqlx::query_as::<_, UsageLineRow>(
        "SELECT t.slug AS tenant_slug, u.provider, u.capability, u.key_source, u.model,
                count(*)::bigint AS calls, sum(u.input_tokens)::bigint AS input_tokens,
                sum(u.output_tokens)::bigint AS output_tokens, sum(u.units)::bigint AS units,
                sum(u.cost_usd)::float8 AS cost_usd,
                count(*) FILTER (WHERE u.cost_usd IS NULL)::bigint AS unpriced_calls
         FROM ai_usage u JOIN tenants t ON t.id = u.tenant_id
         WHERE u.at >= $1 AND u.at < $2
         GROUP BY 1, 2, 3, 4, 5 ORDER BY 1, 2, 3, 4",
    )
    .bind(from)
    .bind(to)
    .fetch_all(conn)
    .await?)
}

// ---------------------------------------------------------------- metering (tenant connection)

#[derive(Debug, Clone)]
pub struct NewUsage<'a> {
    pub provider: &'a str,
    pub capability: &'a str,
    pub key_source: &'a str,
    pub model: Option<&'a str>,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub units: i32,
    pub cost_usd: Option<f64>,
    pub ok: bool,
    pub job_id: Option<Uuid>,
}

pub async fn insert_usage(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    u: &NewUsage<'_>,
) -> Result<(), DbError> {
    sqlx::query(
        "INSERT INTO ai_usage (tenant_id, provider, capability, key_source, model, input_tokens,
                               output_tokens, units, cost_usd, ok, job_id)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9::float8::numeric, $10, $11)",
    )
    .bind(tenant_id)
    .bind(u.provider)
    .bind(u.capability)
    .bind(u.key_source)
    .bind(u.model)
    .bind(u.input_tokens.max(0))
    .bind(u.output_tokens.max(0))
    .bind(u.units.max(0))
    .bind(u.cost_usd)
    .bind(u.ok)
    .bind(u.job_id)
    .execute(conn)
    .await?;
    Ok(())
}

/// Platform-key cost since `from` (this month, for the budget check).
pub async fn platform_cost_since(
    conn: &mut PgConnection,
    from: DateTime<Utc>,
) -> Result<f64, DbError> {
    Ok(sqlx::query_scalar::<_, f64>(
        "SELECT coalesce(sum(cost_usd), 0)::float8 FROM ai_usage WHERE key_source = 'platform' AND at >= $1",
    )
    .bind(from)
    .fetch_one(conn)
    .await?)
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct UsageSummaryRow {
    pub provider: String,
    pub capability: String,
    pub key_source: String,
    pub calls: i64,
    pub failed: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub units: i64,
    pub cost_usd: Option<f64>,
}

pub async fn usage_summary(
    conn: &mut PgConnection,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> Result<Vec<UsageSummaryRow>, DbError> {
    Ok(sqlx::query_as::<_, UsageSummaryRow>(
        "SELECT provider, capability, key_source, count(*)::bigint AS calls,
                count(*) FILTER (WHERE NOT ok)::bigint AS failed,
                sum(input_tokens)::bigint AS input_tokens, sum(output_tokens)::bigint AS output_tokens,
                sum(units)::bigint AS units, sum(cost_usd)::float8 AS cost_usd
         FROM ai_usage WHERE at >= $1 AND at < $2
         GROUP BY 1, 2, 3 ORDER BY 1, 2, 3",
    )
    .bind(from)
    .bind(to)
    .fetch_all(conn)
    .await?)
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct UsageDayRow {
    pub day: chrono::NaiveDate,
    pub calls: i64,
    pub cost_usd: Option<f64>,
}

pub async fn usage_daily(
    conn: &mut PgConnection,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> Result<Vec<UsageDayRow>, DbError> {
    Ok(sqlx::query_as::<_, UsageDayRow>(
        "SELECT (at AT TIME ZONE 'UTC')::date AS day, count(*)::bigint AS calls, sum(cost_usd)::float8 AS cost_usd
         FROM ai_usage WHERE at >= $1 AND at < $2 GROUP BY 1 ORDER BY 1",
    )
    .bind(from)
    .bind(to)
    .fetch_all(conn)
    .await?)
}

// ---------------------------------------------------------------- Jev judgements (tenant connection)

pub async fn set_review(
    conn: &mut PgConnection,
    reel_id: Uuid,
    review: &serde_json::Value,
    quality_score: Option<i16>,
) -> Result<(), DbError> {
    sqlx::query("UPDATE reels SET ai_review = $2, quality_score = $3 WHERE id = $1")
        .bind(reel_id)
        .bind(review)
        .bind(quality_score)
        .execute(conn)
        .await?;
    Ok(())
}

/// The company's most used tags on published reels: the vocabulary Jev connects new reels to.
pub async fn tag_vocabulary(conn: &mut PgConnection, limit: i64) -> Result<Vec<String>, DbError> {
    Ok(sqlx::query_scalar::<_, String>(
        "SELECT tag FROM (SELECT unnest(tags) AS tag FROM reels WHERE status = 'published') t
         GROUP BY tag ORDER BY count(*) DESC, tag LIMIT $1",
    )
    .bind(limit)
    .fetch_all(conn)
    .await?)
}

pub async fn role_fit(
    conn: &mut PgConnection,
    role_key: &str,
) -> Result<Vec<(Uuid, f32)>, DbError> {
    Ok(sqlx::query_as::<_, (Uuid, f32)>(
        "SELECT reel_id, fit FROM reel_role_fit WHERE role_key = $1",
    )
    .bind(role_key)
    .fetch_all(conn)
    .await?)
}

pub async fn upsert_role_fit(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    reel_id: Uuid,
    role_key: &str,
    fit: f32,
    model: Option<&str>,
) -> Result<(), DbError> {
    sqlx::query(
        "INSERT INTO reel_role_fit (tenant_id, reel_id, role_key, fit, model) VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (tenant_id, role_key, reel_id) DO UPDATE SET fit = EXCLUDED.fit, model = EXCLUDED.model, scored_at = now()",
    )
    .bind(tenant_id)
    .bind(reel_id)
    .bind(role_key)
    .bind(fit.clamp(0.0, 1.0))
    .bind(model)
    .execute(conn)
    .await?;
    Ok(())
}

/// Job titles of active people, normalised like [`reelearn_domain::ai::role_key`], busiest first.
pub async fn active_roles(conn: &mut PgConnection, limit: i64) -> Result<Vec<String>, DbError> {
    Ok(sqlx::query_scalar::<_, String>(
        "SELECT k FROM (
           SELECT lower(regexp_replace(btrim(job_title), '\\s+', ' ', 'g')) AS k FROM users
           WHERE status = 'active' AND job_title IS NOT NULL AND btrim(job_title) <> ''
         ) t WHERE length(k) <= 120 GROUP BY k ORDER BY count(*) DESC, k LIMIT $1",
    )
    .bind(limit)
    .fetch_all(conn)
    .await?)
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RoleGapRow {
    pub reel_id: Uuid,
    pub title: String,
    pub topic: String,
    pub tags: Vec<String>,
    pub excerpt: Option<String>,
}

/// Published reels with no relevance judgement yet for `role_key`.
pub async fn role_gaps(
    conn: &mut PgConnection,
    role_key: &str,
    limit: i64,
) -> Result<Vec<RoleGapRow>, DbError> {
    Ok(sqlx::query_as::<_, RoleGapRow>(
        "SELECT r.id AS reel_id, r.title, tp.name AS topic, r.tags, left(r.script, 500) AS excerpt
         FROM reels r JOIN topics tp ON tp.id = r.topic_id
         WHERE r.status = 'published'
           AND NOT EXISTS (SELECT 1 FROM reel_role_fit f WHERE f.reel_id = r.id AND f.role_key = $1)
         ORDER BY r.published_at DESC NULLS LAST LIMIT $2",
    )
    .bind(role_key)
    .bind(limit)
    .fetch_all(conn)
    .await?)
}

pub async fn reel_tags(conn: &mut PgConnection, reel_id: Uuid) -> Result<Vec<String>, DbError> {
    Ok(
        sqlx::query_scalar::<_, Vec<String>>("SELECT tags FROM reels WHERE id = $1")
            .bind(reel_id)
            .fetch_optional(conn)
            .await?
            .unwrap_or_default(),
    )
}
