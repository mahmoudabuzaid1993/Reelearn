//! `/admin/integrations` (secrets write-only, AES-GCM at rest) and `/admin/audit`.

use crate::auth::AuthUser;
use crate::dto::*;
use crate::error::ApiError;
use crate::routes::{page_params, record_audit, Json, Path, Query};
use crate::state::AppState;
use axum::extract::State;
use axum::routing::{get, put};
use axum::Router;
use reelearn_db::repos::{audit, integrations};
use reelearn_domain::cursor::{page, Cursor};
use reelearn_domain::IntegrationKind;
use serde::Deserialize;
use uuid::Uuid;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/admin/integrations", get(list))
        .route("/admin/integrations/{kind}", put(upsert))
        .route("/admin/integrations/{kind}/test", axum::routing::post(test))
        .route("/admin/ai", get(ai_get).put(ai_put))
        .route("/admin/ai/usage", get(ai_usage))
        .route("/admin/audit", get(audit_list))
}

fn public_config(kind: IntegrationKind, config: &serde_json::Value) -> serde_json::Value {
    let mut out = serde_json::Map::new();
    if let Some(obj) = config.as_object() {
        for (k, v) in obj {
            if kind.secret_keys().contains(&k.as_str()) {
                out.insert(k.clone(), serde_json::Value::String("••••".into()));
            } else {
                out.insert(k.clone(), v.clone());
            }
        }
    }
    serde_json::Value::Object(out)
}

/// Decrypt the stored config.
pub fn stored_config(state: &AppState, row: &integrations::IntegrationRow) -> serde_json::Value {
    row.config_enc
        .as_deref()
        .and_then(|b| state.secrets.decrypt_json(b).ok())
        .unwrap_or_else(|| serde_json::json!({}))
}

fn view(state: &AppState, row: &integrations::IntegrationRow) -> Integration {
    let config = stored_config(state, row);
    Integration {
        kind: row.kind,
        status: row.status.clone(),
        last_sync_at: row.last_sync_at,
        last_error: row.last_error.clone(),
        public_config: public_config(row.kind, &config),
    }
}

async fn list(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<Integration>>, ApiError> {
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    let rows = integrations::list(tx.conn()).await?;
    tx.commit().await?;
    let out = IntegrationKind::ALL
        .iter()
        .map(|k| match rows.iter().find(|r| r.kind == *k) {
            Some(r) => view(&state, r),
            None => Integration {
                kind: *k,
                status: "disconnected".into(),
                last_sync_at: None,
                last_error: None,
                public_config: serde_json::json!({}),
            },
        })
        .collect();
    Ok(Json(out))
}

fn parse_kind(kind: &str) -> Result<IntegrationKind, ApiError> {
    kind.parse()
        .map_err(|_| ApiError::NotFound(format!("unknown integration kind {kind}")))
}

async fn upsert(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(kind): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<Integration>, ApiError> {
    let kind = parse_kind(&kind)?;
    let incoming = body
        .as_object()
        .ok_or_else(|| ApiError::BadRequest("body must be a JSON object".into()))?;
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    // Merge over the existing config so a PUT without secrets keeps the stored ones.
    let mut merged = match integrations::find(tx.conn(), kind).await? {
        Some(existing) => stored_config(&state, &existing)
            .as_object()
            .cloned()
            .unwrap_or_default(),
        None => serde_json::Map::new(),
    };
    if let Some(bad) = incoming
        .keys()
        .find(|k| !crate::services::ai::allowed_key(kind, k))
    {
        return Err(ApiError::field(bad, "is not a setting of this integration"));
    }
    for (k, v) in incoming {
        if v.is_null() {
            merged.remove(k);
        } else if v.as_str() == Some("••••") {
            continue; // masked value echoed back by a client: keep the stored secret
        } else {
            merged.insert(k.clone(), v.clone());
        }
    }
    let has_required = kind.required_keys().iter().all(|k| {
        merged
            .get(*k)
            .and_then(|v| v.as_str())
            .map(|s| !s.is_empty())
            .unwrap_or(false)
    });
    let status = if has_required {
        "connected"
    } else {
        "disconnected"
    };
    let blob = state
        .secrets
        .encrypt_json(&serde_json::Value::Object(merged))
        .map_err(ApiError::internal)?;
    let row = integrations::upsert(tx.conn(), auth.tenant_id, kind, status, &blob).await?;
    let keys: Vec<&String> = incoming.keys().collect();
    record_audit(
        &mut tx,
        auth.user_id,
        "integration.update",
        "integration",
        Some(row.id),
        Some(serde_json::json!({ "kind": kind, "keys": keys })),
    )
    .await?;
    tx.commit().await?;
    Ok(Json(view(&state, &row)))
}

#[derive(serde::Serialize)]
struct TestResult {
    ok: bool,
    message: String,
}

async fn test(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(kind): Path<String>,
) -> Result<Json<TestResult>, ApiError> {
    let kind = parse_kind(&kind)?;
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    let row = integrations::find(tx.conn(), kind).await?;
    let result = match row {
        None => TestResult {
            ok: false,
            message: "not configured".into(),
        },
        Some(r) => {
            let config = stored_config(&state, &r);
            let missing: Vec<&str> = kind
                .required_keys()
                .iter()
                .copied()
                .filter(|k| {
                    config
                        .get(*k)
                        .and_then(|v| v.as_str())
                        .map(|s| s.is_empty())
                        .unwrap_or(true)
                })
                .collect();
            if !missing.is_empty() {
                integrations::set_status(
                    tx.conn(),
                    kind,
                    "error",
                    Some(&format!("missing {}", missing.join(", "))),
                )
                .await?;
                TestResult {
                    ok: false,
                    message: format!("missing: {}", missing.join(", ")),
                }
            } else {
                let message = probe(&state, kind, &config).await;
                let ok = message.is_none();
                integrations::set_status(
                    tx.conn(),
                    kind,
                    if ok { "connected" } else { "error" },
                    message.as_deref(),
                )
                .await?;
                TestResult {
                    ok,
                    message: message.unwrap_or_else(|| "credentials present and accepted".into()),
                }
            }
        }
    };
    tx.commit().await?;
    Ok(Json(result))
}

/// Live probe for providers with a cheap authenticated endpoint; `None` means OK.
pub(crate) async fn probe(
    state: &AppState,
    kind: IntegrationKind,
    config: &serde_json::Value,
) -> Option<String> {
    if state.is_development() {
        return None;
    }
    let get = |k: &str| {
        config
            .get(k)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string()
    };
    let base = |default: &str| {
        let b = get("base_url");
        if b.is_empty() {
            default.to_string()
        } else {
            b.trim_end_matches('/').to_string()
        }
    };
    let req = match kind {
        IntegrationKind::AiClaude => state
            .http
            .get("https://api.anthropic.com/v1/models")
            .header("x-api-key", get("api_key"))
            .header("anthropic-version", "2023-06-01"),
        IntegrationKind::AiMuse => state
            .http
            .get(format!("{}/models", base("https://api.meta.ai/v1")))
            .bearer_auth(get("api_key")),
        IntegrationKind::AiTypesafe => state
            .http
            .get(format!("{}/v1/models", base("https://api.typesafe.ai")))
            .bearer_auth(get("api_key")),
        IntegrationKind::Whatsapp => state
            .http
            .get(format!(
                "https://graph.facebook.com/v20.0/{}",
                get("phone_number_id")
            ))
            .bearer_auth(get("access_token")),
        _ => return None,
    };
    match req.send().await {
        Ok(r) if r.status().is_success() => None,
        Ok(r) => Some(format!("provider answered {}", r.status())),
        Err(e) => Some(format!("provider unreachable: {e}")),
    }
}

// ---------------------------------------------------------------- AI: routing, access, usage

#[derive(serde::Serialize)]
struct AiOverview {
    routing: reelearn_domain::ai::AiRouting,
    /// What the platform includes: whether its keys may be used and this month's budget.
    plan: AiPlanView,
    providers: Vec<crate::services::ai::ProviderAccess>,
}

#[derive(serde::Serialize)]
struct AiPlanView {
    platform_keys: bool,
    allowed: Vec<String>,
    monthly_budget_usd: Option<f64>,
    /// Billed platform-key spend this month (markup included), in USD.
    spent_this_month_usd: f64,
}

async fn ai_overview(state: &AppState, tenant_id: Uuid) -> Result<AiOverview, ApiError> {
    let resolved = crate::services::ai::resolve(state, tenant_id).await?;
    let providers = crate::services::ai::access(state, tenant_id).await?;
    Ok(AiOverview {
        routing: resolved.routing.clone(),
        plan: AiPlanView {
            platform_keys: resolved.plan.platform_keys,
            allowed: resolved.plan.allowed.clone(),
            monthly_budget_usd: resolved.plan.monthly_budget_usd,
            spent_this_month_usd: resolved.plan.billable(resolved.platform_spend_this_month),
        },
        providers,
    })
}

async fn ai_get(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<AiOverview>, ApiError> {
    Ok(Json(ai_overview(&state, auth.tenant_id).await?))
}

async fn ai_put(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<reelearn_domain::ai::AiRouting>,
) -> Result<Json<AiOverview>, ApiError> {
    if body.review_threshold > 100 {
        return Err(ApiError::field("review_threshold", "must be 0 to 100"));
    }
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    let tenant = reelearn_db::repos::tenants::get(tx.conn(), auth.tenant_id).await?;
    let mut settings = reelearn_db::settings::TenantSettings::from_json(&tenant.settings);
    let before = settings.ai.clone();
    settings.ai = body.sanitized();
    reelearn_db::repos::tenants::update(
        tx.conn(),
        auth.tenant_id,
        reelearn_db::repos::tenants::TenantPatch {
            settings: Some(serde_json::to_value(&settings).map_err(ApiError::internal)?),
            ..Default::default()
        },
    )
    .await?;
    record_audit(
        &mut tx,
        auth.user_id,
        "ai.routing.update",
        "tenant",
        Some(auth.tenant_id),
        Some(serde_json::json!({ "before": before, "after": settings.ai })),
    )
    .await?;
    tx.commit().await?;
    Ok(Json(ai_overview(&state, auth.tenant_id).await?))
}

#[derive(Deserialize)]
struct UsageQuery {
    /// `YYYY-MM`; this month when absent.
    month: Option<String>,
}

pub(crate) fn month_window(
    month: Option<&str>,
) -> Result<(chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>), ApiError> {
    use crate::services::ai::{month_start, next_month_start};
    use chrono::TimeZone;
    let start = match month {
        None => month_start(chrono::Utc::now()),
        Some(m) => {
            let (y, mo) = m
                .split_once('-')
                .and_then(|(y, mo)| Some((y.parse::<i32>().ok()?, mo.parse::<u32>().ok()?)))
                .ok_or_else(|| ApiError::field("month", "must be YYYY-MM"))?;
            chrono::Utc
                .with_ymd_and_hms(y, mo, 1, 0, 0, 0)
                .single()
                .ok_or_else(|| ApiError::field("month", "must be YYYY-MM"))?
        }
    };
    Ok((start, next_month_start(start)))
}

#[derive(serde::Serialize)]
struct UsageLine {
    provider: String,
    capability: String,
    key_source: String,
    calls: i64,
    failed: i64,
    input_tokens: i64,
    output_tokens: i64,
    units: i64,
    cost_usd: Option<f64>,
}

#[derive(serde::Serialize)]
struct UsageDay {
    day: chrono::NaiveDate,
    calls: i64,
    cost_usd: Option<f64>,
}

#[derive(serde::Serialize)]
struct AiUsage {
    from: chrono::DateTime<chrono::Utc>,
    to: chrono::DateTime<chrono::Utc>,
    lines: Vec<UsageLine>,
    days: Vec<UsageDay>,
    /// Platform-key cost with the plan's markup: what the platform bills the company.
    platform_billable_usd: f64,
    /// Cost of calls on the company's own keys, as priced (paid directly to the vendors).
    company_cost_usd: f64,
}

async fn ai_usage(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(q): Query<UsageQuery>,
) -> Result<Json<AiUsage>, ApiError> {
    use reelearn_db::repos::ai as repo;
    let (from, to) = month_window(q.month.as_deref())?;
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    let plan = repo::plan(tx.conn(), auth.tenant_id).await?;
    let lines = repo::usage_summary(tx.conn(), from, to).await?;
    let days = repo::usage_daily(tx.conn(), from, to).await?;
    tx.commit().await?;
    let sum = |src: &str| {
        lines
            .iter()
            .filter(|l| l.key_source == src)
            .filter_map(|l| l.cost_usd)
            .sum::<f64>()
    };
    Ok(Json(AiUsage {
        from,
        to,
        platform_billable_usd: plan.billable(sum("platform")),
        company_cost_usd: sum("company"),
        lines: lines
            .into_iter()
            .map(|l| UsageLine {
                provider: l.provider,
                capability: l.capability,
                key_source: l.key_source,
                calls: l.calls,
                failed: l.failed,
                input_tokens: l.input_tokens,
                output_tokens: l.output_tokens,
                units: l.units,
                cost_usd: l.cost_usd,
            })
            .collect(),
        days: days
            .into_iter()
            .map(|d| UsageDay {
                day: d.day,
                calls: d.calls,
                cost_usd: d.cost_usd,
            })
            .collect(),
    }))
}

#[derive(Deserialize)]
struct AuditQuery {
    entity: Option<String>,
    entity_id: Option<Uuid>,
    #[serde(flatten)]
    page: PageQuery,
}

async fn audit_list(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(q): Query<AuditQuery>,
) -> Result<Json<Page<AuditEntry>>, ApiError> {
    let (cursor, limit) = page_params(&q.page)?;
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    let rows = audit::list(tx.conn(), q.entity.as_deref(), q.entity_id, cursor, limit).await?;
    tx.commit().await?;
    let (rows, next) = page(rows, limit, |r| Cursor { ts: r.at, id: r.id });
    let items = rows
        .into_iter()
        .map(|r| AuditEntry {
            id: r.id,
            actor: r.actor_id.map(|id| AuditActor {
                id,
                full_name: r.actor_name.clone().unwrap_or_default(),
            }),
            action: r.action,
            entity: r.entity,
            entity_id: r.entity_id,
            diff: r.diff,
            at: r.at,
        })
        .collect();
    Ok(Json(Page {
        items,
        next_cursor: next,
    }))
}
